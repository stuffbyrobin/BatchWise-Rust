//! Container-tracking business logic: asset lifecycle (fill → deliver → return),
//! event logging, and QR-code generation.
//!
//! Port of the Go `internal/tracking/service.go`.

use base64::Engine;
use chrono::Utc;
use uuid::Uuid;

use super::models::{
    Asset, AssetFilter, CreateAssetRequest, DeliverRequest, FillRequest, Log, LogFilter, Page,
    PatchAssetRequest, QrResult, ReturnRequest, SetStatusRequest, UpdateAssetRequest,
};
use super::qr;
use super::repository as repo;
use crate::platform::errors::ApiError;
use crate::state::AppState;

fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .is_some_and(|d| d.is_unique_violation())
}

fn today() -> String {
    Utc::now().date_naive().to_string()
}

/// Creates an asset (status `empty`).
pub async fn create_asset(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateAssetRequest,
) -> Result<Asset, ApiError> {
    match repo::insert_asset(
        &state.pool,
        tenant_id,
        &req.asset_number,
        &req.container_type,
        req.capacity_liters,
        req.deposit_pence,
        req.notes.as_deref(),
    )
    .await
    {
        Ok(a) => Ok(a),
        Err(e) if is_unique_violation(&e) => Err(ApiError::conflict(
            "container_asset",
            "asset_number already exists for this tenant",
        )),
        Err(e) => Err(e.into()),
    }
}

pub async fn list_assets(
    state: &AppState,
    tenant_id: Uuid,
    filter: AssetFilter,
) -> Result<Page<Asset>, ApiError> {
    let order_by = asset_sort(&filter.sort);
    Ok(repo::select_assets(&state.pool, tenant_id, &filter, &order_by).await?)
}

pub async fn get_asset(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<Asset, ApiError> {
    repo::select_asset_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("container_asset"))
}

pub async fn update_asset(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: UpdateAssetRequest,
) -> Result<Asset, ApiError> {
    let mut a = get_asset(state, tenant_id, id).await?;
    a.asset_number = req.asset_number;
    a.container_type = req.container_type;
    a.capacity_liters = req.capacity_liters;
    a.deposit_pence = req.deposit_pence;
    a.notes = req.notes;
    persist(state, &a).await?;
    get_asset(state, tenant_id, id).await
}

pub async fn patch_asset(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchAssetRequest,
) -> Result<Asset, ApiError> {
    let mut a = get_asset(state, tenant_id, id).await?;
    if let Some(v) = req.asset_number {
        a.asset_number = v;
    }
    if let Some(v) = req.container_type {
        a.container_type = v;
    }
    if let Some(v) = req.capacity_liters {
        a.capacity_liters = v;
    }
    if let Some(v) = req.deposit_pence {
        a.deposit_pence = v;
    }
    if req.notes.is_some() {
        a.notes = req.notes;
    }
    persist(state, &a).await?;
    get_asset(state, tenant_id, id).await
}

async fn persist(state: &AppState, a: &Asset) -> Result<(), ApiError> {
    match repo::update_asset_full(&state.pool, a).await {
        Ok(()) => Ok(()),
        Err(e) if is_unique_violation(&e) => Err(ApiError::conflict(
            "container_asset",
            "asset_number already exists for this tenant",
        )),
        Err(e) => Err(e.into()),
    }
}

/// Deletes an asset; blocked if it has logs and is not retired.
pub async fn delete_asset(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    let a = get_asset(state, tenant_id, id).await?;
    if a.status != "retired" && repo::has_logs(&state.pool, id).await? {
        return Err(ApiError::business_rule(
            "container_has_logs",
            "Cannot delete a container that has log entries unless it is retired.",
            Default::default(),
        ));
    }
    repo::delete_asset(&state.pool, tenant_id, id).await?;
    Ok(())
}

/// State-machine helper: load asset, require `from` status, mutate, log, commit.
#[allow(clippy::too_many_arguments)]
async fn lifecycle<F>(
    state: &AppState,
    tenant_id: Uuid,
    user_id: Uuid,
    id: Uuid,
    require_status: Option<&str>,
    require_err: (&str, &str),
    event_type: &str,
    customer_name: Option<String>,
    batch_id: Option<Uuid>,
    notes: Option<String>,
    mutate: F,
) -> Result<Asset, ApiError>
where
    F: FnOnce(&mut Asset),
{
    let mut tx = state.pool.begin().await?;
    let mut a = repo::select_asset_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("container_asset"))?;

    if let Some(req) = require_status {
        if a.status != req {
            return Err(ApiError::business_rule(
                require_err.0,
                require_err.1,
                Default::default(),
            ));
        }
    }
    let from_status = a.status.clone();
    mutate(&mut a);
    let to_status = a.status.clone();

    repo::update_asset_full(&mut *tx, &a).await?;
    repo::insert_log(
        &mut tx,
        &Log {
            id: Uuid::nil(),
            tenant_id,
            container_id: id,
            event_type: event_type.to_string(),
            from_status: Some(from_status),
            to_status: Some(to_status),
            batch_id,
            customer_name,
            notes,
            logged_by_user_id: Some(user_id),
            created_at: Utc::now(),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(a)
}

pub async fn fill(
    state: &AppState,
    tenant_id: Uuid,
    user_id: Uuid,
    id: Uuid,
    req: FillRequest,
) -> Result<Asset, ApiError> {
    let batch_id = req.batch_id;
    lifecycle(
        state,
        tenant_id,
        user_id,
        id,
        Some("empty"),
        (
            "container_not_empty",
            "Container must be empty before filling.",
        ),
        "fill",
        None,
        batch_id,
        req.notes,
        |a| {
            a.status = "filled".to_string();
            a.current_batch_id = batch_id;
            a.last_fill_date = Some(today());
        },
    )
    .await
}

pub async fn deliver(
    state: &AppState,
    tenant_id: Uuid,
    user_id: Uuid,
    id: Uuid,
    req: DeliverRequest,
) -> Result<Asset, ApiError> {
    let customer = req.customer_name.clone();
    lifecycle(
        state,
        tenant_id,
        user_id,
        id,
        Some("filled"),
        (
            "container_not_filled",
            "Container must be filled before delivering.",
        ),
        "deliver",
        Some(customer.clone()),
        None,
        req.notes,
        |a| {
            a.status = "delivered".to_string();
            a.current_customer_name = Some(customer);
        },
    )
    .await
}

pub async fn return_asset(
    state: &AppState,
    tenant_id: Uuid,
    user_id: Uuid,
    id: Uuid,
    req: ReturnRequest,
) -> Result<Asset, ApiError> {
    lifecycle(
        state,
        tenant_id,
        user_id,
        id,
        Some("delivered"),
        (
            "container_not_delivered",
            "Container must be delivered before returning.",
        ),
        "return",
        None,
        None,
        req.notes,
        |a| {
            a.status = "empty".to_string();
            a.current_batch_id = None;
            a.current_customer_name = None;
            a.last_return_date = Some(today());
        },
    )
    .await
}

pub async fn set_status(
    state: &AppState,
    tenant_id: Uuid,
    user_id: Uuid,
    id: Uuid,
    req: SetStatusRequest,
) -> Result<Asset, ApiError> {
    let to = req.to_status.clone();
    lifecycle(
        state,
        tenant_id,
        user_id,
        id,
        None,
        ("", ""),
        "status_change",
        None,
        None,
        req.notes,
        |a| a.status = to,
    )
    .await
}

pub async fn list_logs(
    state: &AppState,
    tenant_id: Uuid,
    filter: LogFilter,
) -> Result<Page<Log>, ApiError> {
    let order_by = "created_at DESC";
    Ok(repo::select_logs(&state.pool, tenant_id, &filter, order_by).await?)
}

pub async fn get_log(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<Log, ApiError> {
    repo::select_log_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("container_log"))
}

/// Generates a QR code for a container. Variant `a` encodes a JSON id payload;
/// variant `b` encodes a public URL.
pub async fn generate_qr(
    state: &AppState,
    tenant_id: Uuid,
    container_id: Uuid,
    variant: &str,
) -> Result<QrResult, ApiError> {
    get_asset(state, tenant_id, container_id).await?; // tenant ownership check

    let payload = match variant {
        "a" => format!("{{\"id\":\"{container_id}\"}}"),
        "b" => format!("{}/c/{}", state.config.app_base_url, container_id),
        _ => return Err(ApiError::validation("variant", "must be 'a' or 'b'")),
    };
    let png = qr::encode_png(&payload)?;
    Ok(QrResult {
        container_id: container_id.to_string(),
        variant: variant.to_string(),
        payload,
        png_base64: base64::engine::general_purpose::STANDARD.encode(&png),
    })
}

pub async fn count_assets_by_statuses(
    state: &AppState,
    tenant_id: Uuid,
    statuses: &[String],
) -> Result<i64, ApiError> {
    Ok(repo::count_assets_by_statuses(&state.pool, tenant_id, statuses).await?)
}

fn asset_sort(sort: &str) -> String {
    let spec = if sort.is_empty() { "-created_at" } else { sort };
    let desc = spec.starts_with('-');
    let col = match spec.trim_start_matches('-') {
        "asset_number" => "asset_number",
        "status" => "status",
        _ => "created_at",
    };
    format!("{col} {}", if desc { "DESC" } else { "ASC" })
}
