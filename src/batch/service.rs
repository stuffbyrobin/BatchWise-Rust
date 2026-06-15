//! Batch business logic: recipe snapshotting, calendar-event generation, the
//! status FSM, and deferred inventory deduction on the `planned → brewing`
//! transition.
//!
//! Port of the Go `internal/batch/service.go`.

use chrono::{Datelike, NaiveDate, TimeZone, Utc};
use uuid::Uuid;

use super::models::{
    Batch, BatchIngredient, BatchRecipeSnapshot, CreateRequest, CreateResult, ListFilter,
    PatchIngredientsRequest, UpdateRequest,
};
use super::repository::{self as repo, BatchMutable, NewBatch};
use crate::calendar::models::EventWrite;
use crate::calendar::service as calendar_svc;
use crate::inventory::models::DeductRequest;
use crate::inventory::service as inventory_svc;
use crate::platform::errors::ApiError;
use crate::recipe::service as recipe_svc;
use crate::state::AppState;
use crate::yeastkinetics::service as kinetics_svc;

fn system_tenant_id() -> Uuid {
    Uuid::nil()
}

/// FSM: allowed next statuses for each batch status.
fn allowed_next(status: &str) -> &'static [&'static str] {
    match status {
        "planned" => &["brewing", "cancelled"],
        "brewing" => &["fermenting", "cancelled"],
        "fermenting" => &["conditioning", "cancelled", "spoiled"],
        "conditioning" => &["packaging", "cancelled", "spoiled"],
        "packaging" => &["completed", "cancelled", "spoiled"],
        "completed" => &["spoiled"],
        _ => &[],
    }
}

fn round4(v: Option<f64>) -> Option<f64> {
    v.map(|x| (x * 10000.0).round() / 10000.0)
}

fn parse_date(s: &Option<String>) -> Result<Option<NaiveDate>, ApiError> {
    match s {
        None => Ok(None),
        Some(v) if v.is_empty() => Ok(None),
        Some(v) => NaiveDate::parse_from_str(v, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| ApiError::validation("brew_date", "invalid date format (YYYY-MM-DD)")),
    }
}

fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .is_some_and(|d| d.is_unique_violation())
}

/// Creates a batch, snapshots its recipe, and generates calendar events.
pub async fn create(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateRequest,
) -> Result<CreateResult, ApiError> {
    // Recipe: tenant first, then the system tenant.
    let rec = match recipe_svc::get(state, tenant_id, req.recipe_id).await {
        Ok(r) => r,
        Err(e) if e.status() == axum::http::StatusCode::NOT_FOUND => {
            recipe_svc::get(state, system_tenant_id(), req.recipe_id)
                .await
                .map_err(|_| ApiError::not_found("recipe"))?
        }
        Err(e) => return Err(e),
    };

    let snapshot = BatchRecipeSnapshot {
        schema_version: 1,
        recipe_id: rec.recipe.id,
        name: rec.recipe.name.clone(),
        r#type: rec.recipe.r#type.clone(),
        batch_size_liters: rec.recipe.batch_size_liters,
        boil_size_liters: rec.recipe.boil_size_liters,
        boil_time_minutes: rec.recipe.boil_time_minutes,
        efficiency_pct: rec.recipe.efficiency_pct,
        calc_og: rec.recipe.calc_og,
        calc_fg: rec.recipe.calc_fg,
        calc_abv_pct: rec.recipe.calc_abv_pct,
        calc_ibu: rec.recipe.calc_ibu,
        calc_color_ebc: rec.recipe.calc_color_ebc,
        fermentables: rec.fermentables,
        hops: rec.hops,
        yeasts: rec.yeasts,
        mash_steps: rec.mash_steps,
    };

    let initial_status = req
        .initial_status
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "planned".to_string());
    let brew_date = parse_date(&req.brew_date)?;

    // Replace an existing *planned* batch with the same number; else conflict.
    if let Some(existing) =
        repo::select_by_batch_number(&state.pool, tenant_id, &req.batch_number).await?
    {
        if existing.status != "planned" {
            return Err(ApiError::conflict(
                "batch",
                "batch_number already exists for this tenant",
            ));
        }
    }

    let mut tx = state.pool.begin().await?;

    if let Some(existing) =
        repo::select_by_batch_number(&state.pool, tenant_id, &req.batch_number).await?
    {
        if existing.status == "planned" {
            repo::delete_by_id(&mut *tx, tenant_id, existing.id).await?;
        }
    }

    let new_batch = NewBatch {
        recipe_id: Some(req.recipe_id),
        batch_number: req.batch_number.clone(),
        name: req.name.clone(),
        status: initial_status,
        brew_date,
        notes: req.notes.clone(),
        duty_status: "suspended".to_string(),
        target_og: round4(rec.recipe.calc_og),
        target_fg: round4(rec.recipe.calc_fg),
        snapshot,
    };
    let batch = match repo::insert(&mut *tx, tenant_id, &new_batch).await {
        Ok(b) => b,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict(
                "batch",
                "batch_number already exists for this tenant",
            ));
        }
        Err(e) => return Err(e.into()),
    };

    repo::increment_tenant_batch_number(&mut *tx, tenant_id).await?;

    let events = build_events(state, tenant_id, &batch).await;
    let created_events = calendar_svc::create_for_batch_tx(&mut tx, tenant_id, &events).await?;

    tx.commit().await?;
    Ok(CreateResult {
        batch,
        generated_calendar_events: created_events,
    })
}

/// Builds the calendar-event timeline for a new batch.
async fn build_events(state: &AppState, tenant_id: Uuid, batch: &Batch) -> Vec<EventWrite> {
    let brew_date = batch
        .brew_date
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| Utc::now().date_naive());
    let brew_day = Utc
        .with_ymd_and_hms(
            brew_date.year(),
            brew_date.month(),
            brew_date.day(),
            9,
            0,
            0,
        )
        .single()
        .unwrap_or_else(Utc::now);

    let title = |what: &str| format!("{what} --- {} --- {}", batch.batch_number, batch.name);
    let ev = |event_type: &str, title: String, start| EventWrite {
        batch_id: Some(batch.id),
        event_type: event_type.to_string(),
        title,
        start_time: start,
        end_time: None,
        status: "pending".to_string(),
        notify_minutes_before: None,
        notes: Some(String::new()),
    };

    let primary_yeast_id = batch
        .batch_recipe_snapshot
        .yeasts
        .first()
        .and_then(|y| y.yeast_id);

    if let Some(yeast_id) = primary_yeast_id {
        if let Ok(Some(k)) =
            kinetics_svc::find_closest_for_yeast(&state.pool, tenant_id, yeast_id, 18.0).await
        {
            let lag_hours = k.lag_phase_hours.unwrap_or(24);
            let ferm_complete = brew_day
                + chrono::Duration::hours(lag_hours as i64)
                + chrono::Duration::days(k.primary_fermentation_days as i64);
            let cond_complete = ferm_complete + chrono::Duration::days(k.conditioning_days as i64);
            return vec![
                ev("brew_day", title("Brew day"), brew_day),
                ev(
                    "fermentation_complete",
                    title("Fermentation complete"),
                    ferm_complete,
                ),
                ev(
                    "condition_complete",
                    title("Condition complete"),
                    cond_complete,
                ),
                ev("package", title("Package"), cond_complete),
            ];
        }
    }

    // Default timeline: brew day + package in 28 days.
    let package_time = brew_day + chrono::Duration::days(28);
    vec![
        ev("brew_day", title("Brew day"), brew_day),
        ev("package", title("Package"), package_time),
    ]
}

/// Lists batches.
pub async fn list(
    state: &AppState,
    tenant_id: Uuid,
    filter: ListFilter,
) -> Result<super::models::Page<Batch>, ApiError> {
    let order_by = build_sort(&filter.sort);
    Ok(repo::select_list(&state.pool, tenant_id, &filter, &order_by).await?)
}

/// Fetches a batch.
pub async fn get(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<Batch, ApiError> {
    repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("batch"))
}

/// Updates a batch's mutable fields (not allowed in terminal states).
pub async fn update(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: UpdateRequest,
) -> Result<Batch, ApiError> {
    let existing = get(state, tenant_id, id).await?;
    if matches!(
        existing.status.as_str(),
        "completed" | "cancelled" | "spoiled"
    ) {
        return Err(ApiError::business_rule(
            "batch_terminal_status",
            "Cannot modify a batch in terminal status.",
            Default::default(),
        ));
    }

    let brew_date = match &req.brew_date {
        Some(_) => parse_date(&req.brew_date)?,
        None => parse_date(&existing.brew_date)?,
    };
    let package_date = match &req.package_date {
        Some(_) => parse_date(&req.package_date)?,
        None => parse_date(&existing.package_date)?,
    };

    let m = BatchMutable {
        name: req.name.unwrap_or(existing.name),
        brew_date,
        package_date,
        target_og: req.target_og.or(existing.target_og),
        actual_og: req.actual_og.or(existing.actual_og),
        target_fg: req.target_fg.or(existing.target_fg),
        actual_fg: req.actual_fg.or(existing.actual_fg),
        actual_volume_liters: req.actual_volume_liters.or(existing.actual_volume_liters),
        notes: req.notes.or(existing.notes),
    };

    let mut tx = state.pool.begin().await?;
    repo::update_mutable(&mut *tx, tenant_id, id, &m).await?;
    tx.commit().await?;
    get(state, tenant_id, id).await
}

/// Deletes a batch (only planned or cancelled).
pub async fn delete(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    let existing = get(state, tenant_id, id).await?;
    if existing.status != "planned" && existing.status != "cancelled" {
        return Err(ApiError::business_rule(
            "batch_not_deletable",
            "Only planned or cancelled batches can be deleted.",
            Default::default(),
        ));
    }
    let mut tx = state.pool.begin().await?;
    repo::delete_by_id(&mut *tx, tenant_id, id).await?;
    tx.commit().await?;
    Ok(())
}

/// Transitions batch status, deducting inventory on `planned → brewing`.
pub async fn transition(
    state: &AppState,
    tenant_id: Uuid,
    user_id: Uuid,
    id: Uuid,
    to_status: &str,
) -> Result<Batch, ApiError> {
    let mut tx = state.pool.begin().await?;
    let batch = repo::select_for_update(&mut *tx, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("batch"))?;

    let allowed = allowed_next(&batch.status);
    if !allowed.contains(&to_status) {
        let mut details = std::collections::BTreeMap::new();
        details.insert("from_status".into(), serde_json::json!(batch.status));
        details.insert("to_status".into(), serde_json::json!(to_status));
        details.insert("allowed_next".into(), serde_json::json!(allowed));
        return Err(ApiError::business_rule(
            "invalid_status_transition",
            &format!(
                "Cannot transition batch from {} to {to_status}.",
                batch.status
            ),
            details,
        ));
    }

    if batch.status == "planned" && to_status == "brewing" {
        deduct_for_brewing(
            &mut tx,
            state.config.allow_overdraft,
            tenant_id,
            user_id,
            &batch,
        )
        .await?;
    }

    repo::update_status(&mut *tx, tenant_id, id, to_status).await?;
    tx.commit().await?;
    get(state, tenant_id, id).await
}

/// Deducts all snapshot ingredients (FIFO) and records batch_ingredients.
async fn deduct_for_brewing(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    overdraft: bool,
    tenant_id: Uuid,
    user_id: Uuid,
    batch: &Batch,
) -> Result<(), ApiError> {
    let snap = &batch.batch_recipe_snapshot;

    // (type, name, amount, unit, preferred_lot_id)
    let mut items: Vec<(&str, String, f64, String, Option<Uuid>)> = Vec::new();
    for f in &snap.fermentables {
        items.push((
            "fermentable",
            f.name.clone(),
            f.amount,
            f.unit.clone(),
            f.inventory_lot_id,
        ));
    }
    for h in &snap.hops {
        items.push((
            "hop",
            h.name.clone(),
            h.amount,
            h.unit.clone(),
            h.inventory_lot_id,
        ));
    }
    for y in &snap.yeasts {
        items.push((
            "yeast",
            y.name.clone(),
            y.amount,
            y.unit.clone(),
            y.inventory_lot_id,
        ));
    }

    for (typ, name, amount, unit, preferred) in items {
        let req = DeductRequest {
            r#type: typ.to_string(),
            name,
            amount,
            unit,
            reference_type: "batch".to_string(),
            reference_id: Some(batch.id),
            notes: None,
            preferred_lot_id: preferred,
        };
        let result =
            inventory_svc::deduct_in_tx(&mut *tx, overdraft, tenant_id, user_id, &req).await?;
        if let Some(first) = result.allocations.first() {
            repo::insert_batch_ingredient(
                &mut *tx,
                &BatchIngredient {
                    batch_id: batch.id,
                    ingredient_id: first.lot_id,
                    amount_deducted: first.amount_deducted,
                    unit: result.unit.clone(),
                    cost_pence: 0,
                },
            )
            .await?;
        }
    }
    Ok(())
}

/// Replaces the snapshot's ingredient lists (not allowed in terminal states).
pub async fn patch_ingredients(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchIngredientsRequest,
) -> Result<Batch, ApiError> {
    let mut tx = state.pool.begin().await?;
    let existing = repo::select_for_update(&mut *tx, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("batch"))?;
    if matches!(
        existing.status.as_str(),
        "completed" | "cancelled" | "spoiled"
    ) {
        return Err(ApiError::business_rule(
            "batch_terminal_status",
            "Cannot modify ingredients of a batch in terminal status.",
            Default::default(),
        ));
    }

    let mut snapshot = existing.batch_recipe_snapshot;
    snapshot.fermentables = req.fermentables;
    snapshot.hops = req.hops;
    snapshot.yeasts = req.yeasts;

    repo::update_snapshot(&mut *tx, tenant_id, id, &snapshot).await?;
    tx.commit().await?;
    get(state, tenant_id, id).await
}

/// Returns a count per status (all eight statuses present).
pub async fn status_breakdown(
    state: &AppState,
    tenant_id: Uuid,
) -> Result<std::collections::HashMap<String, i64>, ApiError> {
    Ok(repo::count_by_status(&state.pool, tenant_id).await?)
}

fn build_sort(sort: &str) -> String {
    let spec = if sort.is_empty() { "-created_at" } else { sort };
    let desc = spec.starts_with('-');
    let col = match spec.trim_start_matches('-') {
        "brew_date" => "brew_date",
        "batch_number" => "batch_number",
        "name" => "name",
        _ => "created_at",
    };
    format!("{col} {}", if desc { "DESC" } else { "ASC" })
}
