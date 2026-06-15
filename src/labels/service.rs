//! Label-record business logic: creation, listing, retrieval, patching,
//! approval, and deletion.
//!
//! Port of the Go `internal/compliance/labels/service.go`. On creation the
//! record is auto-populated from the batch's recipe snapshot, the tenant's
//! identity/address, the computed allergen declaration (via
//! [`crate::allergens`]), and ABV-derived nutrition values (via
//! [`crate::pkg::nutrition`]). Approved records are immutable. Create, update,
//! approve, and delete each record a fire-and-forget compliance audit event.

use chrono::NaiveDate;
use serde_json::json;
use uuid::Uuid;

use super::models::{CreateRequest, LabelRecord, ListFilter, Page, PatchRequest};
use super::repository::{self as repo, LabelInsert};
use crate::pkg::{allergen, nutrition};
use crate::platform::errors::ApiError;
use crate::state::AppState;
use crate::{allergens, audit, tenant};

/// Creates a draft label record for a batch, auto-populating compliance and
/// voluntary fields.
pub async fn create(
    state: &AppState,
    tenant_id: Uuid,
    actor_id: Option<Uuid>,
    req: CreateRequest,
) -> Result<LabelRecord, ApiError> {
    let batch = repo::select_batch_info(&state.pool, tenant_id, req.batch_id)
        .await?
        .ok_or_else(|| ApiError::not_found("batch"))?;

    let tn = tenant::repository::get_by_id(&state.pool, tenant_id)
        .await?
        .ok_or_else(|| ApiError::not_found("tenant"))?;

    // Allergens are computed from the batch's recipe, if any; failures are
    // tolerated (Go discards the error and falls back to an empty list).
    let allergen_list = match batch.recipe_id {
        Some(recipe_id) => allergens::compute_for_recipe(state, tenant_id, actor_id, recipe_id)
            .await
            .map(|r| r.allergens)
            .unwrap_or_default(),
        None => Vec::new(),
    };

    let abv = batch.abv_percent;

    let responsible_party = if !tn.address.is_empty() {
        format!("{}, {}", tn.tenant_name, tn.address)
    } else {
        tn.tenant_name.clone()
    };

    let serving_vol = req.serving_volume_ml.unwrap_or(req.net_volume_ml) as f64;

    let (energy_kj, energy_kcal, units) = if abv > 0.0 {
        (
            Some(nutrition::energy_kj_per_100ml(abv)),
            Some(nutrition::energy_kcal_per_100ml(abv)),
            Some(nutrition::alcohol_units(abv, serving_vol)),
        )
    } else {
        (None, None, None)
    };

    let ins = LabelInsert {
        batch_id: req.batch_id,
        status: "draft".to_string(),
        product_name: batch.product_name,
        abv_percent: abv,
        allergens: allergen_list,
        net_volume_ml: req.net_volume_ml,
        responsible_party,
        country_of_origin: tn.country,
        best_before_date: None,
        lot_identifier: batch.batch_number,
        ingredient_list: None,
        energy_kj_per_100ml: energy_kj,
        energy_kcal_per_100ml: energy_kcal,
        alcohol_units_per_serving: units,
        serving_volume_ml: req.serving_volume_ml,
    };

    let rec = repo::insert(&state.pool, tenant_id, &ins).await?;

    audit::service::write(
        &state.pool,
        audit::models::WriteRequest {
            tenant_id,
            event_type: audit::models::EVENT_LABEL_CREATED,
            entity_type: "label_record",
            entity_id: Some(rec.id),
            actor_user_id: actor_id,
            event_data: json!({
                "product_name": rec.product_name,
                "abv_percent": rec.abv_percent,
                "allergens": rec.allergens,
                "net_volume_ml": rec.net_volume_ml,
                "lot_identifier": rec.lot_identifier,
                "status": rec.status,
            }),
        },
    )
    .await;
    Ok(rec)
}

/// Lists label records.
pub async fn list(
    state: &AppState,
    tenant_id: Uuid,
    filter: ListFilter,
) -> Result<Page<LabelRecord>, ApiError> {
    Ok(repo::select_list(&state.pool, tenant_id, &filter).await?)
}

/// Fetches a label record, 404 if absent or cross-tenant.
pub async fn get(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<LabelRecord, ApiError> {
    repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("label_record"))
}

/// Returns the most recently updated approved label record for a batch, or 404.
pub async fn latest_approved_for_batch(
    state: &AppState,
    tenant_id: Uuid,
    batch_id: Uuid,
) -> Result<LabelRecord, ApiError> {
    repo::select_latest_approved_for_batch(&state.pool, tenant_id, batch_id)
        .await?
        .ok_or_else(|| ApiError::not_found("label_record"))
}

/// Applies a partial update to a label record. Approved records cannot be
/// modified; transitioning to `approved` requires all mandatory fields.
pub async fn patch(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    actor_id: Option<Uuid>,
    req: PatchRequest,
) -> Result<LabelRecord, ApiError> {
    let mut rec = repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("label_record"))?;

    if rec.status == "approved" {
        return Err(ApiError::business_rule(
            "label_record_approved",
            "Approved label records cannot be modified.",
            Default::default(),
        ));
    }

    // Capture audit metadata from the request before its fields are consumed.
    let changed_fields = changed_fields(&req);
    let patch_values = patch_values(&req);
    let approving = req.status.as_deref() == Some("approved");

    if let Some(v) = req.product_name {
        rec.product_name = v;
    }
    if let Some(v) = req.abv_percent {
        rec.abv_percent = v;
    }
    if let Some(v) = req.allergens {
        let norm = allergen::normalise(&v);
        allergen::validate(&norm).map_err(|e| ApiError::validation("allergens", &e))?;
        rec.allergens = norm;
    }
    if let Some(v) = req.net_volume_ml {
        rec.net_volume_ml = v;
    }
    if let Some(v) = req.responsible_party {
        rec.responsible_party = v;
    }
    if let Some(v) = req.country_of_origin {
        rec.country_of_origin = v;
    }
    if let Some(v) = req.best_before_date {
        NaiveDate::parse_from_str(&v, "%Y-%m-%d").map_err(|_| {
            ApiError::validation("best_before_date", "must be ISO 8601 date (YYYY-MM-DD)")
        })?;
        rec.best_before_date = Some(v);
    }
    if let Some(v) = req.lot_identifier {
        rec.lot_identifier = v;
    }
    if let Some(v) = req.ingredient_list {
        rec.ingredient_list = Some(v);
    }
    if let Some(v) = req.energy_kj_per_100ml {
        rec.energy_kj_per_100ml = Some(v);
    }
    if let Some(v) = req.energy_kcal_per_100ml {
        rec.energy_kcal_per_100ml = Some(v);
    }
    if let Some(v) = req.alcohol_units_per_serving {
        rec.alcohol_units_per_serving = Some(v);
    }
    if let Some(v) = req.serving_volume_ml {
        rec.serving_volume_ml = Some(v);
    }

    if let Some(status) = req.status {
        if status == "approved" {
            validate_for_approval(&rec)?;
        }
        rec.status = status;
    }

    repo::update_full(&state.pool, tenant_id, &rec).await?;

    let (event_type, event_data) = if approving {
        (
            audit::models::EVENT_LABEL_APPROVED,
            json!({
                "product_name": rec.product_name,
                "lot_identifier": rec.lot_identifier,
            }),
        )
    } else {
        (
            audit::models::EVENT_LABEL_UPDATED,
            json!({
                "fields_changed": changed_fields,
                "new_values": patch_values,
            }),
        )
    };
    audit::service::write(
        &state.pool,
        audit::models::WriteRequest {
            tenant_id,
            event_type,
            entity_type: "label_record",
            entity_id: Some(id),
            actor_user_id: actor_id,
            event_data,
        },
    )
    .await;

    repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("label_record"))
}

/// Deletes a label record. Approved records cannot be deleted.
pub async fn delete(
    state: &AppState,
    tenant_id: Uuid,
    actor_id: Option<Uuid>,
    id: Uuid,
) -> Result<(), ApiError> {
    let rec = repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("label_record"))?;

    if rec.status == "approved" {
        return Err(ApiError::business_rule(
            "label_record_approved",
            "Approved label records cannot be deleted.",
            Default::default(),
        ));
    }

    if !repo::delete_by_id(&state.pool, tenant_id, id).await? {
        return Err(ApiError::not_found("label_record"));
    }

    audit::service::write(
        &state.pool,
        audit::models::WriteRequest {
            tenant_id,
            event_type: audit::models::EVENT_LABEL_DELETED,
            entity_type: "label_record",
            entity_id: Some(id),
            actor_user_id: actor_id,
            event_data: json!({
                "product_name": rec.product_name,
                "lot_identifier": rec.lot_identifier,
                "status_at_delete": rec.status,
            }),
        },
    )
    .await;
    Ok(())
}

/// The list of label fields touched by a patch request (for the audit log).
fn changed_fields(req: &PatchRequest) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if req.product_name.is_some() {
        fields.push("product_name");
    }
    if req.abv_percent.is_some() {
        fields.push("abv_percent");
    }
    if req.allergens.is_some() {
        fields.push("allergens");
    }
    if req.net_volume_ml.is_some() {
        fields.push("net_volume_ml");
    }
    if req.status.is_some() {
        fields.push("status");
    }
    if req.best_before_date.is_some() {
        fields.push("best_before_date");
    }
    if req.lot_identifier.is_some() {
        fields.push("lot_identifier");
    }
    fields
}

/// The subset of patched values recorded in the audit log.
fn patch_values(req: &PatchRequest) -> serde_json::Value {
    let mut vals = serde_json::Map::new();
    if let Some(v) = &req.product_name {
        vals.insert("product_name".to_string(), json!(v));
    }
    if let Some(v) = req.abv_percent {
        vals.insert("abv_percent".to_string(), json!(v));
    }
    if let Some(v) = &req.status {
        vals.insert("status".to_string(), json!(v));
    }
    if let Some(v) = &req.lot_identifier {
        vals.insert("lot_identifier".to_string(), json!(v));
    }
    serde_json::Value::Object(vals)
}

/// Ensures all mandatory label fields are present before approval.
fn validate_for_approval(rec: &LabelRecord) -> Result<(), ApiError> {
    let mut missing: Vec<&str> = Vec::new();
    if rec.product_name.trim().is_empty() {
        missing.push("product_name");
    }
    if rec.abv_percent <= 0.0 {
        missing.push("abv_percent");
    }
    if rec.net_volume_ml <= 0 {
        missing.push("net_volume_ml");
    }
    if rec.responsible_party.trim().is_empty() {
        missing.push("responsible_party");
    }
    if rec.lot_identifier.trim().is_empty() {
        missing.push("lot_identifier");
    }
    if !missing.is_empty() {
        let mut details = std::collections::BTreeMap::new();
        details.insert("missing_fields".to_string(), serde_json::json!(missing));
        return Err(ApiError::business_rule(
            "label_record_incomplete",
            &format!(
                "Cannot approve: missing required fields: {}",
                missing.join(", ")
            ),
            details,
        ));
    }
    Ok(())
}
