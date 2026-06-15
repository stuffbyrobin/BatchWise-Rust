//! Sales business logic: customer CRUD, the order lifecycle FSM
//! (draft → confirmed → fulfilled → invoiced, plus cancelled), order items, and
//! duty-event crystallisation on fulfilment.
//!
//! Port of the Go `internal/sales/service.go`. Money stays `i64` pence; the
//! order total is recomputed by the DB (a sub-select over `order_items`), so the
//! line-item arithmetic is identical to the Go source.

use chrono::Utc;
use uuid::Uuid;

use super::models::{
    CancelOrderRequest, CreateCustomerRequest, CreateItemRequest, CreateOrderRequest, Customer,
    CustomerFilter, DutyEvent, DutyEventFilter, FulfillOrderRequest, Order, OrderFilter, OrderItem,
    Page, PatchCustomerRequest, PatchOrderRequest,
};
use super::repository as repo;
use crate::batch::service as batch_svc;
use crate::pkg::duty;
use crate::pkg::gravity;
use crate::platform::errors::ApiError;
use crate::state::AppState;
use crate::tenant::repository as tenant_repo;

fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .is_some_and(|d| d.is_unique_violation())
}

fn today() -> String {
    Utc::now().date_naive().to_string()
}

/// FSM helper: valid next statuses for an order, used in error details.
fn allowed_next(status: &str) -> &'static [&'static str] {
    match status {
        "draft" => &["confirmed", "cancelled"],
        "confirmed" => &["fulfilled", "cancelled"],
        "fulfilled" => &["invoiced"],
        _ => &[],
    }
}

fn fsm_error(from: &str, to: &str) -> ApiError {
    let allowed = allowed_next(from);
    let mut details = std::collections::BTreeMap::new();
    details.insert("from_status".into(), serde_json::json!(from));
    details.insert("to_status".into(), serde_json::json!(to));
    details.insert("allowed_next".into(), serde_json::json!(allowed));
    ApiError::business_rule(
        "invalid_status_transition",
        &format!("Cannot transition order from {from} to {to}."),
        details,
    )
}

// ---- customers ----

/// Creates a customer (default country `GB`).
pub async fn create_customer(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateCustomerRequest,
) -> Result<Customer, ApiError> {
    let country = if req.country.is_empty() {
        "GB".to_string()
    } else {
        req.country
    };
    let c = Customer {
        id: Uuid::nil(),
        tenant_id,
        name: req.name,
        contact_name: req.contact_name,
        email: req.email,
        phone: req.phone,
        address_line1: req.address_line1,
        address_line2: req.address_line2,
        city: req.city,
        postcode: req.postcode,
        country,
        notes: req.notes,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    match repo::insert_customer(&state.pool, tenant_id, &c).await {
        Ok(created) => Ok(created),
        Err(e) if is_unique_violation(&e) => Err(ApiError::conflict(
            "name",
            "A customer with this name already exists",
        )),
        Err(e) => Err(e.into()),
    }
}

pub async fn list_customers(
    state: &AppState,
    tenant_id: Uuid,
    filter: CustomerFilter,
) -> Result<Page<Customer>, ApiError> {
    let order_by = customer_sort(&filter.sort);
    Ok(repo::select_customers(&state.pool, tenant_id, &filter, &order_by).await?)
}

pub async fn get_customer(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Customer, ApiError> {
    repo::select_customer_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("customer"))
}

/// Replaces all mutable fields (PUT). Empty country defaults to `GB`.
pub async fn replace_customer(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: CreateCustomerRequest,
) -> Result<Customer, ApiError> {
    let mut c = get_customer(state, tenant_id, id).await?;
    let country = if req.country.is_empty() {
        "GB".to_string()
    } else {
        req.country
    };
    c.name = req.name;
    c.contact_name = req.contact_name;
    c.email = req.email;
    c.phone = req.phone;
    c.address_line1 = req.address_line1;
    c.address_line2 = req.address_line2;
    c.city = req.city;
    c.postcode = req.postcode;
    c.country = country;
    c.notes = req.notes;
    persist_customer(state, &c).await
}

/// Patches the provided customer fields (PATCH).
pub async fn patch_customer(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchCustomerRequest,
) -> Result<Customer, ApiError> {
    let mut c = get_customer(state, tenant_id, id).await?;
    if let Some(v) = req.name {
        c.name = v;
    }
    if req.contact_name.is_some() {
        c.contact_name = req.contact_name;
    }
    if req.email.is_some() {
        c.email = req.email;
    }
    if req.phone.is_some() {
        c.phone = req.phone;
    }
    if req.address_line1.is_some() {
        c.address_line1 = req.address_line1;
    }
    if req.address_line2.is_some() {
        c.address_line2 = req.address_line2;
    }
    if req.city.is_some() {
        c.city = req.city;
    }
    if req.postcode.is_some() {
        c.postcode = req.postcode;
    }
    if let Some(v) = req.country {
        c.country = v;
    }
    if req.notes.is_some() {
        c.notes = req.notes;
    }
    persist_customer(state, &c).await
}

async fn persist_customer(state: &AppState, c: &Customer) -> Result<Customer, ApiError> {
    match repo::update_customer(&state.pool, c).await {
        Ok(Some(updated)) => Ok(updated),
        Ok(None) => Err(ApiError::not_found("customer")),
        Err(e) if is_unique_violation(&e) => Err(ApiError::conflict(
            "name",
            "A customer with this name already exists",
        )),
        Err(e) => Err(e.into()),
    }
}

/// Deletes a customer; blocked while it has non-cancelled orders.
pub async fn delete_customer(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    get_customer(state, tenant_id, id).await?;
    if repo::customer_has_active_orders(&state.pool, tenant_id, id).await? {
        let mut details = std::collections::BTreeMap::new();
        details.insert("customer_id".into(), serde_json::json!(id));
        return Err(ApiError::business_rule(
            "customer_has_orders",
            "Customer has active orders — cancel all orders before deleting",
            details,
        ));
    }
    repo::delete_customer(&state.pool, tenant_id, id).await?;
    Ok(())
}

// ---- orders ----

/// Creates a draft order, assigning the next `ORD-N` number atomically.
pub async fn create_order(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateOrderRequest,
) -> Result<Order, ApiError> {
    // Verify customer exists (tenant-scoped).
    repo::select_customer_by_id(&state.pool, tenant_id, req.customer_id)
        .await?
        .ok_or_else(|| ApiError::not_found("customer"))?;

    let order_date = req
        .order_date
        .filter(|d| !d.is_empty())
        .unwrap_or_else(today);

    let mut tx = state.pool.begin().await?;
    let n = repo::get_and_increment_order_number(&mut tx, tenant_id).await?;

    let order = Order {
        id: Uuid::nil(),
        tenant_id,
        customer_id: req.customer_id,
        customer_name: String::new(),
        order_number: format!("ORD-{n}"),
        status: "draft".to_string(),
        order_date,
        fulfillment_date: None,
        notes: req.notes,
        total_price_pence: 0,
        items: Vec::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let created = repo::insert_order(&mut tx, &order).await?;
    tx.commit().await?;
    Ok(created)
}

pub async fn list_orders(
    state: &AppState,
    tenant_id: Uuid,
    filter: OrderFilter,
) -> Result<Page<Order>, ApiError> {
    let order_by = order_sort(&filter.sort);
    Ok(repo::select_orders(&state.pool, tenant_id, &filter, &order_by).await?)
}

pub async fn get_order(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<Order, ApiError> {
    repo::select_order_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("order"))
}

/// Patches order date / notes; only allowed while in draft.
pub async fn patch_order(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchOrderRequest,
) -> Result<Order, ApiError> {
    let mut o = get_order(state, tenant_id, id).await?;
    if o.status != "draft" {
        return Err(not_draft(
            "Order fields can only be edited while in draft status",
            &o.status,
        ));
    }
    if let Some(d) = req.order_date {
        o.order_date = d;
    }
    if req.notes.is_some() {
        o.notes = req.notes;
    }
    let mut tx = state.pool.begin().await?;
    repo::update_order(&mut tx, &o).await?;
    tx.commit().await?;
    get_order(state, tenant_id, id).await
}

/// Deletes an order; only allowed while in draft.
pub async fn delete_order(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    let o = get_order(state, tenant_id, id).await?;
    if o.status != "draft" {
        return Err(not_draft(
            "Order can only be deleted when in draft status",
            &o.status,
        ));
    }
    if !repo::delete_order(&state.pool, tenant_id, id).await? {
        return Err(ApiError::not_found("order"));
    }
    Ok(())
}

fn not_draft(message: &str, current_status: &str) -> ApiError {
    let mut details = std::collections::BTreeMap::new();
    details.insert("current_status".into(), serde_json::json!(current_status));
    ApiError::business_rule("order_not_draft", message, details)
}

// ---- order items ----

/// Adds an item to a draft order. Verifies any linked batch exists.
pub async fn add_item(
    state: &AppState,
    tenant_id: Uuid,
    order_id: Uuid,
    req: CreateItemRequest,
) -> Result<OrderItem, ApiError> {
    let o = get_order(state, tenant_id, order_id).await?;
    if o.status != "draft" {
        return Err(not_draft(
            "Items can only be added to draft orders",
            &o.status,
        ));
    }
    if let Some(batch_id) = req.batch_id {
        batch_svc::get(state, tenant_id, batch_id)
            .await
            .map_err(|_| ApiError::not_found("batch"))?;
    }
    let item = OrderItem {
        id: Uuid::nil(),
        tenant_id,
        order_id,
        batch_id: req.batch_id,
        product_name: req.product_name,
        volume_liters: req.volume_liters,
        unit_price_pence: req.unit_price_pence,
        quantity: req.quantity.unwrap_or(1),
        total_price_pence: 0,
        notes: req.notes,
        created_at: Utc::now(),
    };
    Ok(repo::insert_item(&state.pool, &item).await?)
}

/// Replaces an item on a draft order. Verifies any linked batch exists.
pub async fn replace_item(
    state: &AppState,
    tenant_id: Uuid,
    order_id: Uuid,
    item_id: Uuid,
    req: CreateItemRequest,
) -> Result<OrderItem, ApiError> {
    let o = get_order(state, tenant_id, order_id).await?;
    if o.status != "draft" {
        return Err(not_draft(
            "Items can only be edited on draft orders",
            &o.status,
        ));
    }
    let mut existing = repo::select_item_by_id(&state.pool, tenant_id, order_id, item_id)
        .await?
        .ok_or_else(|| ApiError::not_found("order_item"))?;
    if let Some(batch_id) = req.batch_id {
        batch_svc::get(state, tenant_id, batch_id)
            .await
            .map_err(|_| ApiError::not_found("batch"))?;
    }
    existing.batch_id = req.batch_id;
    existing.product_name = req.product_name;
    existing.volume_liters = req.volume_liters;
    existing.unit_price_pence = req.unit_price_pence;
    existing.quantity = req.quantity.unwrap_or(existing.quantity);
    existing.notes = req.notes;
    repo::update_item(&state.pool, &existing)
        .await?
        .ok_or_else(|| ApiError::not_found("order_item"))
}

/// Removes an item from a draft order.
pub async fn delete_item(
    state: &AppState,
    tenant_id: Uuid,
    order_id: Uuid,
    item_id: Uuid,
) -> Result<(), ApiError> {
    let o = get_order(state, tenant_id, order_id).await?;
    if o.status != "draft" {
        return Err(not_draft(
            "Items can only be removed from draft orders",
            &o.status,
        ));
    }
    if !repo::delete_item(&state.pool, tenant_id, order_id, item_id).await? {
        return Err(ApiError::not_found("order_item"));
    }
    Ok(())
}

// ---- order FSM ----

/// draft → confirmed. Requires at least one item.
pub async fn confirm_order(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<Order, ApiError> {
    let o = get_order(state, tenant_id, id).await?;
    if o.status != "draft" {
        return Err(fsm_error(&o.status, "confirmed"));
    }
    if o.items.is_empty() {
        let mut details = std::collections::BTreeMap::new();
        details.insert("order_id".into(), serde_json::json!(id));
        return Err(ApiError::business_rule(
            "order_has_no_items",
            "Order must have at least one item before it can be confirmed",
            details,
        ));
    }
    set_status(state, tenant_id, o, "confirmed").await
}

/// confirmed → fulfilled. Crystallises a `sale` duty event per batch-linked item.
pub async fn fulfill_order(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: FulfillOrderRequest,
) -> Result<Order, ApiError> {
    let mut o = get_order(state, tenant_id, id).await?;
    if o.status != "confirmed" {
        return Err(fsm_error(&o.status, "fulfilled"));
    }

    let tenant = tenant_repo::get_by_id(&state.pool, tenant_id)
        .await?
        .ok_or_else(|| ApiError::not_found("tenant"))?;

    let fulfill_date = req
        .fulfillment_date
        .filter(|d| !d.is_empty())
        .unwrap_or_else(today);

    let mut tx = state.pool.begin().await?;

    // One duty event per item that is linked to a batch.
    for item in &o.items {
        let Some(batch_id) = item.batch_id else {
            continue;
        };
        let batch = match batch_svc::get(state, tenant_id, batch_id).await {
            Ok(b) => b,
            Err(_) => {
                tracing::warn!(
                    %batch_id, order_id = %id,
                    "batch not found for duty event; skipping"
                );
                continue;
            }
        };

        let mut abv_pct = 0.0;
        if let (Some(og), Some(fg)) = (batch.actual_og, batch.actual_fg) {
            if let Ok(abv) = gravity::calculate_abv(og, fg) {
                abv_pct = abv;
            }
        }
        if abv_pct == 0.0 {
            tracing::warn!(
                %batch_id,
                "batch missing actual_og/actual_fg for duty calculation; using abv=0"
            );
        }

        let volume_liters = item.volume_liters * item.quantity as f64;
        // Go fails open (logs and returns 0) for unsupported jurisdictions; this
        // port surfaces an Err which we treat as zero duty.
        let duty_pence = duty::calculate_duty(&tenant.country, volume_liters, abv_pct).unwrap_or(0);

        let event = DutyEvent {
            id: Uuid::nil(),
            tenant_id,
            order_id: id,
            batch_id: Some(batch_id),
            event_type: "sale".to_string(),
            volume_liters,
            abv_pct,
            duty_pence,
            jurisdiction: tenant.country.clone(),
            crystallised_at: Utc::now(),
            created_at: Utc::now(),
        };
        repo::insert_duty_event(&mut tx, &event).await?;
    }

    o.status = "fulfilled".to_string();
    o.fulfillment_date = Some(fulfill_date);
    if req.notes.is_some() {
        o.notes = req.notes;
    }
    repo::update_order(&mut tx, &o).await?;
    tx.commit().await?;
    get_order(state, tenant_id, id).await
}

/// fulfilled → invoiced.
pub async fn invoice_order(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<Order, ApiError> {
    let o = get_order(state, tenant_id, id).await?;
    if o.status != "fulfilled" {
        return Err(fsm_error(&o.status, "invoiced"));
    }
    set_status(state, tenant_id, o, "invoiced").await
}

/// draft|confirmed → cancelled.
pub async fn cancel_order(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: CancelOrderRequest,
) -> Result<Order, ApiError> {
    let mut o = get_order(state, tenant_id, id).await?;
    if o.status != "draft" && o.status != "confirmed" {
        return Err(fsm_error(&o.status, "cancelled"));
    }
    if req.notes.is_some() {
        o.notes = req.notes;
    }
    o.status = "cancelled".to_string();
    let mut tx = state.pool.begin().await?;
    repo::update_order(&mut tx, &o).await?;
    tx.commit().await?;
    get_order(state, tenant_id, id).await
}

/// Persists a status change in a transaction and re-reads the order.
async fn set_status(
    state: &AppState,
    tenant_id: Uuid,
    mut o: Order,
    status: &str,
) -> Result<Order, ApiError> {
    o.status = status.to_string();
    let mut tx = state.pool.begin().await?;
    repo::update_order(&mut tx, &o).await?;
    tx.commit().await?;
    get_order(state, tenant_id, o.id).await
}

// ---- duty events ----

pub async fn list_duty_events(
    state: &AppState,
    tenant_id: Uuid,
    filter: DutyEventFilter,
) -> Result<Page<DutyEvent>, ApiError> {
    let order_by = duty_event_sort(&filter.sort);
    Ok(repo::select_duty_events(&state.pool, tenant_id, &filter, &order_by).await?)
}

pub async fn get_duty_event(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<DutyEvent, ApiError> {
    repo::select_duty_event_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("duty_event"))
}

/// Total revenue for fulfilled/invoiced items tagged to a batch.
pub async fn sum_revenue_for_batch(
    state: &AppState,
    tenant_id: Uuid,
    batch_id: Uuid,
) -> Result<i64, ApiError> {
    Ok(repo::sum_revenue_for_batch(&state.pool, tenant_id, batch_id).await?)
}

// ---- sort builders ----

fn customer_sort(sort: &str) -> String {
    let desc = sort.starts_with('-');
    let col = sort.trim_start_matches('-');
    match col {
        "name" | "created_at" => format!("{col} {}", if desc { "DESC" } else { "ASC" }),
        _ => "name ASC".to_string(),
    }
}

fn order_sort(sort: &str) -> String {
    let desc = sort.starts_with('-');
    let col = sort.trim_start_matches('-');
    match col {
        "order_date" | "order_number" | "created_at" => {
            format!("o.{col} {}", if desc { "DESC" } else { "ASC" })
        }
        _ => "o.order_date DESC".to_string(),
    }
}

fn duty_event_sort(sort: &str) -> String {
    let desc = sort.starts_with('-');
    let col = sort.trim_start_matches('-');
    match col {
        "crystallised_at" | "created_at" => {
            format!("{col} {}", if desc { "DESC" } else { "ASC" })
        }
        _ => "crystallised_at DESC".to_string(),
    }
}
