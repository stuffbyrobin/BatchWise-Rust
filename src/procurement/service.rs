//! Procurement business logic: supplier and purchase-order lifecycle.
//!
//! Port of the Go `internal/procurement/service.go`.

use std::collections::BTreeMap;

use serde_json::json;
use uuid::Uuid;

use super::models::{
    CreateLineRequest, CreatePORequest, CreateSupplierRequest, POFilter, Page, PatchLineRequest,
    PatchPORequest, PatchSupplierRequest, PurchaseOrder, PurchaseOrderLine, ReceiveRequest,
    Supplier, SupplierFilter,
};
use super::repository as repo;
use crate::platform::errors::ApiError;
use crate::state::AppState;

/// True if the error is the supplier-name unique-constraint violation.
fn is_duplicate_name(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .is_some_and(|d| d.is_unique_violation())
}

// ---- suppliers ----

pub async fn create_supplier(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateSupplierRequest,
) -> Result<Supplier, ApiError> {
    match repo::insert_supplier(
        &state.pool,
        tenant_id,
        &req.name,
        req.contact_name.as_deref(),
        req.email.as_deref(),
        req.phone.as_deref(),
        req.website.as_deref(),
        req.notes.as_deref(),
    )
    .await
    {
        Ok(s) => Ok(s),
        Err(e) if is_duplicate_name(&e) => Err(ApiError::conflict(
            "supplier",
            "a supplier with this name already exists",
        )),
        Err(e) => Err(e.into()),
    }
}

pub async fn list_suppliers(
    state: &AppState,
    tenant_id: Uuid,
    filter: SupplierFilter,
) -> Result<Page<Supplier>, ApiError> {
    Ok(repo::select_suppliers(&state.pool, tenant_id, &filter).await?)
}

pub async fn get_supplier(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<Supplier, ApiError> {
    repo::select_supplier_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("supplier"))
}

pub async fn patch_supplier(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchSupplierRequest,
) -> Result<Supplier, ApiError> {
    let mut sup = get_supplier(state, tenant_id, id).await?;
    if let Some(name) = req.name {
        sup.name = name;
    }
    if let Some(v) = req.contact_name {
        sup.contact_name = v;
    }
    if let Some(v) = req.email {
        sup.email = v;
    }
    if let Some(v) = req.phone {
        sup.phone = v;
    }
    if let Some(v) = req.website {
        sup.website = v;
    }
    if let Some(v) = req.notes {
        sup.notes = v;
    }
    match repo::update_supplier(
        &state.pool,
        tenant_id,
        id,
        &sup.name,
        sup.contact_name.as_deref(),
        sup.email.as_deref(),
        sup.phone.as_deref(),
        sup.website.as_deref(),
        sup.notes.as_deref(),
    )
    .await
    {
        Ok(Some(s)) => Ok(s),
        Ok(None) => Err(ApiError::not_found("supplier")),
        Err(e) if is_duplicate_name(&e) => Err(ApiError::conflict(
            "supplier",
            "a supplier with this name already exists",
        )),
        Err(e) => Err(e.into()),
    }
}

pub async fn delete_supplier(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    if repo::supplier_has_pos(&state.pool, tenant_id, id).await? {
        return Err(ApiError::business_rule(
            "supplier_has_orders",
            "cannot delete a supplier that has purchase orders",
            BTreeMap::new(),
        ));
    }
    if !repo::delete_supplier(&state.pool, tenant_id, id).await? {
        return Err(ApiError::not_found("supplier"));
    }
    Ok(())
}

// ---- purchase orders ----

pub async fn create_po(
    state: &AppState,
    tenant_id: Uuid,
    req: CreatePORequest,
) -> Result<PurchaseOrder, ApiError> {
    // Validate supplier before acquiring a transaction connection.
    let sup = repo::select_supplier_by_id(&state.pool, tenant_id, req.supplier_id)
        .await?
        .ok_or_else(|| ApiError::not_found("supplier"))?;

    let mut tx = state.pool.begin().await?;
    let n = repo::get_and_increment_po_number(&mut tx, tenant_id).await?;
    let po_number = format!("PO-{n:05}");

    let po = repo::insert_po(
        &mut tx,
        tenant_id,
        req.supplier_id,
        &sup.name,
        &po_number,
        "draft",
        req.expected_delivery.as_deref(),
        req.notes.as_deref(),
    )
    .await?;
    tx.commit().await?;
    Ok(po)
}

pub async fn list_pos(
    state: &AppState,
    tenant_id: Uuid,
    filter: POFilter,
) -> Result<Page<PurchaseOrder>, ApiError> {
    Ok(repo::select_pos(&state.pool, tenant_id, &filter).await?)
}

pub async fn get_po(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<PurchaseOrder, ApiError> {
    repo::select_po_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("purchase_order"))
}

/// Allowed status transitions, matching the Go `validateTransition` map.
fn allowed_transitions(from: &str) -> Vec<&'static str> {
    match from {
        "draft" => vec!["sent", "cancelled"],
        "sent" => vec!["cancelled", "partially_received", "received"],
        "partially_received" => vec!["received"],
        _ => vec![],
    }
}

fn validate_transition(po: &PurchaseOrder, to_status: &str) -> Result<(), ApiError> {
    let targets = allowed_transitions(&po.status);
    if !targets.contains(&to_status) {
        let mut details = BTreeMap::new();
        details.insert("from_status".to_string(), json!(po.status));
        details.insert("to_status".to_string(), json!(to_status));
        details.insert("allowed_next".to_string(), json!(targets));
        return Err(ApiError::business_rule(
            "invalid_status_transition",
            &format!(
                "cannot transition purchase order from \"{}\" to \"{}\"",
                po.status, to_status
            ),
            details,
        ));
    }
    if to_status == "sent" && po.lines.is_empty() {
        return Err(ApiError::business_rule(
            "no_lines",
            "cannot mark a purchase order as sent with no lines",
            BTreeMap::new(),
        ));
    }
    Ok(())
}

pub async fn patch_po(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchPORequest,
) -> Result<PurchaseOrder, ApiError> {
    let mut po = get_po(state, tenant_id, id).await?;

    if let Some(status) = &req.status {
        validate_transition(&po, status)?;
        po.status = status.clone();
    }
    if let Some(v) = req.expected_delivery {
        po.expected_delivery = v;
    }
    if let Some(v) = req.notes {
        po.notes = v;
    }

    repo::update_po(
        &state.pool,
        tenant_id,
        id,
        &po.status,
        po.expected_delivery.as_deref(),
        po.notes.as_deref(),
    )
    .await?;
    get_po(state, tenant_id, id).await
}

pub async fn delete_po(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    let po = get_po(state, tenant_id, id).await?;
    if po.status != "draft" {
        return Err(ApiError::business_rule(
            "po_not_draft",
            "purchase order can only be deleted while in draft status",
            BTreeMap::new(),
        ));
    }
    if !repo::delete_po(&state.pool, tenant_id, id).await? {
        return Err(ApiError::not_found("purchase_order"));
    }
    Ok(())
}

// ---- lines ----

pub async fn add_line(
    state: &AppState,
    tenant_id: Uuid,
    po_id: Uuid,
    req: CreateLineRequest,
) -> Result<PurchaseOrderLine, ApiError> {
    let po = get_po(state, tenant_id, po_id).await?;
    if po.status != "draft" {
        return Err(ApiError::business_rule(
            "po_not_draft",
            "lines can only be added to draft purchase orders",
            BTreeMap::new(),
        ));
    }
    let currency = if req.unit_cost_currency.is_empty() {
        "GBP"
    } else {
        &req.unit_cost_currency
    };
    Ok(repo::insert_line(
        &state.pool,
        po_id,
        &req.ingredient_type,
        &req.ingredient_name,
        req.quantity,
        &req.unit,
        req.unit_cost_pence,
        currency,
    )
    .await?)
}

pub async fn patch_line(
    state: &AppState,
    tenant_id: Uuid,
    po_id: Uuid,
    line_id: Uuid,
    req: PatchLineRequest,
) -> Result<PurchaseOrderLine, ApiError> {
    let po = get_po(state, tenant_id, po_id).await?;
    if po.status != "draft" {
        return Err(ApiError::business_rule(
            "po_not_draft",
            "lines can only be updated on draft purchase orders",
            BTreeMap::new(),
        ));
    }
    let mut line = repo::select_line_by_id(&state.pool, po_id, line_id)
        .await?
        .ok_or_else(|| ApiError::not_found("purchase_order_line"))?;

    if let Some(v) = req.ingredient_type {
        line.ingredient_type = v;
    }
    if let Some(v) = req.ingredient_name {
        line.ingredient_name = v;
    }
    if let Some(v) = req.quantity {
        line.quantity = v;
    }
    if let Some(v) = req.unit {
        line.unit = v;
    }
    if let Some(v) = req.unit_cost_pence {
        line.unit_cost_pence = v;
    }
    if let Some(v) = req.unit_cost_currency {
        line.unit_cost_currency = v;
    }

    repo::update_line(
        &state.pool,
        line_id,
        &line.ingredient_type,
        &line.ingredient_name,
        line.quantity,
        &line.unit,
        line.unit_cost_pence,
        &line.unit_cost_currency,
    )
    .await?;
    repo::select_line_by_id(&state.pool, po_id, line_id)
        .await?
        .ok_or_else(|| ApiError::not_found("purchase_order_line"))
}

pub async fn delete_line(
    state: &AppState,
    tenant_id: Uuid,
    po_id: Uuid,
    line_id: Uuid,
) -> Result<(), ApiError> {
    let po = get_po(state, tenant_id, po_id).await?;
    if po.status != "draft" {
        return Err(ApiError::business_rule(
            "po_not_draft",
            "lines can only be deleted from draft purchase orders",
            BTreeMap::new(),
        ));
    }
    repo::select_line_by_id(&state.pool, po_id, line_id)
        .await?
        .ok_or_else(|| ApiError::not_found("purchase_order_line"))?;
    repo::delete_line(&state.pool, line_id).await?;
    Ok(())
}

// ---- receipt ----

pub async fn receive_po(
    state: &AppState,
    tenant_id: Uuid,
    po_id: Uuid,
    req: ReceiveRequest,
) -> Result<PurchaseOrder, ApiError> {
    let mut po = get_po(state, tenant_id, po_id).await?;

    if po.status != "sent" && po.status != "partially_received" {
        return Err(ApiError::business_rule(
            "invalid_status_transition",
            "purchase order must be in sent or partially_received status to receive",
            BTreeMap::new(),
        ));
    }

    for rl in &req.lines {
        let Some(line) = po.lines.iter_mut().find(|l| l.id == rl.line_id) else {
            return Err(ApiError::not_found("purchase_order_line"));
        };
        repo::update_line_received_qty(&state.pool, rl.line_id, rl.received_quantity).await?;
        line.received_quantity = Some(rl.received_quantity);
    }

    // Determine new status.
    let mut all_received = true;
    let mut any_received = false;
    for l in &po.lines {
        let rq = l.received_quantity.unwrap_or(0.0);
        if rq > 0.0 {
            any_received = true;
        }
        if rq < l.quantity {
            all_received = false;
        }
    }
    if all_received && any_received {
        po.status = "received".to_string();
    } else if any_received {
        po.status = "partially_received".to_string();
    }

    repo::update_po(
        &state.pool,
        tenant_id,
        po_id,
        &po.status,
        po.expected_delivery.as_deref(),
        po.notes.as_deref(),
    )
    .await?;
    get_po(state, tenant_id, po_id).await
}
