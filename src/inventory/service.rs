//! Inventory business logic, including FIFO-by-best-before-date deduction.
//!
//! Port of the Go `internal/inventory/service.go`.

use chrono::NaiveDate;
use sqlx::PgConnection;
use uuid::Uuid;

use super::models::{
    AllocationEntry, CreateRequest, DeductRequest, DeductResult, Ingredient, ListFilter,
    MovementFilter, Page, PatchRequest, StockInRequest, StockMovement, SummaryFilter, SummaryRow,
};
use super::repository::{self as repo, IngredientWrite, MovementWrite};
use crate::pkg::allergen;
use crate::platform::errors::ApiError;
use crate::state::AppState;

fn parse_bbd(s: &Option<String>) -> Result<Option<NaiveDate>, ApiError> {
    match s {
        None => Ok(None),
        Some(v) => NaiveDate::parse_from_str(v, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| {
                ApiError::validation("best_before_date", "invalid date format (YYYY-MM-DD)")
            }),
    }
}

fn normalise_allergens(tokens: &[String]) -> Result<Vec<String>, ApiError> {
    if tokens.is_empty() {
        return Ok(vec![]);
    }
    let norm = allergen::normalise(tokens);
    allergen::validate(&norm).map_err(|e| ApiError::validation("allergens", &e))?;
    Ok(norm)
}

fn default_currency(c: &str) -> String {
    if c.is_empty() {
        "GBP".to_string()
    } else {
        c.to_string()
    }
}

fn write_from_create(req: &CreateRequest) -> Result<IngredientWrite, ApiError> {
    Ok(IngredientWrite {
        r#type: req.r#type.clone(),
        name: req.name.clone(),
        amount: req.amount,
        unit: req.unit.clone(),
        lot_number: req.lot_number.clone(),
        best_before_date: parse_bbd(&req.best_before_date)?,
        cost_pence: req.cost_pence,
        cost_currency: default_currency(&req.cost_currency),
        supplier: req.supplier.clone(),
        origin: req.origin.clone(),
        color_ebc: req.color_ebc,
        alpha_acid_pct: req.alpha_acid_pct,
        attenuation_pct: req.attenuation_pct,
        allergens: normalise_allergens(&req.allergens)?,
        notes: req.notes.clone(),
    })
}

fn write_from_ingredient(i: &Ingredient) -> Result<IngredientWrite, ApiError> {
    Ok(IngredientWrite {
        r#type: i.r#type.clone(),
        name: i.name.clone(),
        amount: i.amount,
        unit: i.unit.clone(),
        lot_number: i.lot_number.clone(),
        best_before_date: parse_bbd(&i.best_before_date)?,
        cost_pence: i.cost_pence,
        cost_currency: default_currency(&i.cost_currency),
        supplier: i.supplier.clone(),
        origin: i.origin.clone(),
        color_ebc: i.color_ebc,
        alpha_acid_pct: i.alpha_acid_pct,
        attenuation_pct: i.attenuation_pct,
        allergens: i.allergens.clone(),
        notes: i.notes.clone(),
    })
}

fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .is_some_and(|d| d.is_unique_violation())
}

/// Creates a lot and records the opening `stock_in` movement.
pub async fn create(
    state: &AppState,
    tenant_id: Uuid,
    user_id: Uuid,
    req: CreateRequest,
) -> Result<Ingredient, ApiError> {
    let w = write_from_create(&req)?;
    let mut tx = state.pool.begin().await?;

    let lot = match repo::insert(&mut *tx, tenant_id, &w).await {
        Ok(l) => l,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict(
                "ingredient",
                "lot_number already exists for this tenant",
            ));
        }
        Err(e) => return Err(e.into()),
    };

    repo::insert_movement(
        &mut *tx,
        &MovementWrite {
            tenant_id,
            ingredient_id: lot.id,
            amount_delta: lot.amount,
            balance_after: lot.amount,
            reference_type: "stock_in".to_string(),
            reference_id: None,
            notes: None,
            created_by_user_id: Some(user_id),
        },
    )
    .await?;

    tx.commit().await?;
    Ok(lot)
}

/// Lists lots for a tenant.
pub async fn list(
    state: &AppState,
    tenant_id: Uuid,
    filter: ListFilter,
) -> Result<Page<Ingredient>, ApiError> {
    let order_by = build_ing_sort(&filter.sort)?;
    Ok(repo::select_list(&state.pool, tenant_id, &filter, &order_by).await?)
}

/// Fetches a lot, returning 404 if absent or cross-tenant.
pub async fn get(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<Ingredient, ApiError> {
    repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("ingredient"))
}

/// Replaces a lot (PUT semantics).
pub async fn replace(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: CreateRequest,
) -> Result<Ingredient, ApiError> {
    let w = write_from_create(&req)?;
    repo::update_full(&state.pool, tenant_id, id, &w)
        .await?
        .ok_or_else(|| ApiError::not_found("ingredient"))
}

/// Applies a partial update. `lot_number` is immutable.
pub async fn patch(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchRequest,
) -> Result<Ingredient, ApiError> {
    if req.lot_number.is_some() {
        return Err(ApiError::validation(
            "lot_number",
            "lot_number is immutable after creation",
        ));
    }
    let mut ing = repo::select_by_id(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("ingredient"))?;

    if let Some(v) = req.r#type {
        ing.r#type = v;
    }
    if let Some(v) = req.name {
        ing.name = v;
    }
    if let Some(v) = req.amount {
        ing.amount = v;
    }
    if let Some(v) = req.unit {
        ing.unit = v;
    }
    if req.best_before_date.is_some() {
        ing.best_before_date = req.best_before_date;
    }
    if let Some(v) = req.cost_pence {
        ing.cost_pence = v;
    }
    if let Some(v) = req.cost_currency {
        ing.cost_currency = v;
    }
    if req.supplier.is_some() {
        ing.supplier = req.supplier;
    }
    if req.origin.is_some() {
        ing.origin = req.origin;
    }
    if req.color_ebc.is_some() {
        ing.color_ebc = req.color_ebc;
    }
    if req.alpha_acid_pct.is_some() {
        ing.alpha_acid_pct = req.alpha_acid_pct;
    }
    if req.attenuation_pct.is_some() {
        ing.attenuation_pct = req.attenuation_pct;
    }
    if let Some(tokens) = req.allergens {
        ing.allergens = normalise_allergens(&tokens)?;
    }
    if req.notes.is_some() {
        ing.notes = req.notes;
    }

    let w = write_from_ingredient(&ing)?;
    repo::update_full(&state.pool, tenant_id, id, &w)
        .await?
        .ok_or_else(|| ApiError::not_found("ingredient"))
}

/// Deletes a lot unless it is referenced by a batch.
pub async fn delete(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    let mut tx = state.pool.begin().await?;
    if repo::is_referenced_by_batch(&mut *tx, id).await? {
        return Err(ApiError::business_rule(
            "lot_referenced_by_batch",
            "Ingredient lot is referenced by one or more batches and cannot be deleted.",
            Default::default(),
        ));
    }
    if !repo::delete_by_id(&mut *tx, tenant_id, id).await? {
        return Err(ApiError::not_found("ingredient"));
    }
    tx.commit().await?;
    Ok(())
}

/// Appends stock to a lot and records the movement.
pub async fn append_stock(
    state: &AppState,
    tenant_id: Uuid,
    user_id: Uuid,
    id: Uuid,
    req: StockInRequest,
) -> Result<Ingredient, ApiError> {
    let mut tx = state.pool.begin().await?;
    let mut ing = repo::select_by_id_for_update(&mut *tx, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("ingredient"))?;

    if let Some(c) = req.cost_pence {
        ing.cost_pence = c;
    }
    ing.amount += req.amount;

    let w = write_from_ingredient(&ing)?;
    let updated = repo::update_full(&mut *tx, tenant_id, id, &w)
        .await?
        .ok_or_else(|| ApiError::not_found("ingredient"))?;

    repo::insert_movement(
        &mut *tx,
        &MovementWrite {
            tenant_id,
            ingredient_id: id,
            amount_delta: req.amount,
            balance_after: updated.amount,
            reference_type: "stock_in".to_string(),
            reference_id: None,
            notes: req.notes,
            created_by_user_id: Some(user_id),
        },
    )
    .await?;

    tx.commit().await?;
    Ok(updated)
}

/// FIFO deduction in its own transaction.
pub async fn deduct(
    state: &AppState,
    tenant_id: Uuid,
    user_id: Uuid,
    req: DeductRequest,
) -> Result<DeductResult, ApiError> {
    let mut tx = state.pool.begin().await?;
    let result = deduct_in_tx(
        &mut tx,
        state.config.allow_overdraft,
        tenant_id,
        user_id,
        &req,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}

/// FIFO deduction within the caller's transaction. Reused by the batch module
/// when a batch transitions to `brewing`.
pub async fn deduct_in_tx(
    conn: &mut PgConnection,
    allow_overdraft: bool,
    tenant_id: Uuid,
    user_id: Uuid,
    req: &DeductRequest,
) -> Result<DeductResult, ApiError> {
    let mut lots =
        repo::select_for_deduct(&mut *conn, tenant_id, &req.r#type, &req.name, &req.unit).await?;

    // Bubble a preferred lot to the front of the FIFO queue.
    if let Some(pref) = req.preferred_lot_id {
        if let Some(pos) = lots.iter().position(|l| l.id == pref) {
            let chosen = lots.remove(pos);
            lots.insert(0, chosen);
        }
    }

    if lots.is_empty() {
        return Err(insufficient_stock(req.amount, 0.0, req.amount, &req.unit));
    }

    let available: f64 = lots.iter().map(|l| l.amount).sum();
    let mut remaining = req.amount;
    let mut allocations: Vec<AllocationEntry> = Vec::new();

    for lot in &lots {
        if remaining <= 0.0 {
            break;
        }
        let deduct_amt = remaining.min(lot.amount);
        let new_amount = lot.amount - deduct_amt;
        repo::update_amount(&mut *conn, lot.id, new_amount).await?;
        allocations.push(AllocationEntry {
            lot_id: lot.id,
            lot_number: lot.lot_number.clone(),
            best_before_date: lot.best_before_date.clone(),
            amount_deducted: deduct_amt,
            remaining_in_lot: new_amount,
        });
        record_movement(
            &mut *conn,
            tenant_id,
            user_id,
            lot.id,
            -deduct_amt,
            new_amount,
            req,
        )
        .await?;
        remaining -= deduct_amt;
    }

    let mut warning = None;
    if remaining > 0.0 {
        if !allow_overdraft {
            return Err(insufficient_stock(
                req.amount, available, remaining, &req.unit,
            ));
        }
        // Drive the oldest lot negative.
        let oldest = &lots[0];
        let new_amount = oldest.amount - remaining;
        repo::update_amount(&mut *conn, oldest.id, new_amount).await?;
        allocations.push(AllocationEntry {
            lot_id: oldest.id,
            lot_number: oldest.lot_number.clone(),
            best_before_date: oldest.best_before_date.clone(),
            amount_deducted: remaining,
            remaining_in_lot: new_amount,
        });
        record_movement(
            &mut *conn, tenant_id, user_id, oldest.id, -remaining, new_amount, req,
        )
        .await?;
        warning = Some("negative_balance".to_string());
        remaining = 0.0;
    }

    Ok(DeductResult {
        requested_amount: req.amount,
        deducted_amount: req.amount - remaining,
        unit: req.unit.clone(),
        allocations,
        warning,
    })
}

async fn record_movement(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    ingredient_id: Uuid,
    amount_delta: f64,
    balance_after: f64,
    req: &DeductRequest,
) -> Result<(), ApiError> {
    // A manual deduction (no reference_type) records as "manual" to satisfy the
    // stock_movements.reference_type check constraint.
    let reference_type = if req.reference_type.is_empty() {
        "manual".to_string()
    } else {
        req.reference_type.clone()
    };
    repo::insert_movement(
        conn,
        &MovementWrite {
            tenant_id,
            ingredient_id,
            amount_delta,
            balance_after,
            reference_type,
            reference_id: req.reference_id,
            notes: req.notes.clone(),
            created_by_user_id: Some(user_id),
        },
    )
    .await?;
    Ok(())
}

fn insufficient_stock(requested: f64, available: f64, shortage: f64, unit: &str) -> ApiError {
    let mut details = std::collections::BTreeMap::new();
    details.insert("requested_amount".into(), serde_json::json!(requested));
    details.insert("available_amount".into(), serde_json::json!(available));
    details.insert("shortage_amount".into(), serde_json::json!(shortage));
    details.insert("unit".into(), serde_json::json!(unit));
    ApiError::business_rule(
        "insufficient_stock",
        "Insufficient stock to fulfil deduction.",
        details,
    )
}

/// Aggregated inventory summary.
pub async fn summary(
    state: &AppState,
    tenant_id: Uuid,
    filter: SummaryFilter,
) -> Result<Page<SummaryRow>, ApiError> {
    Ok(repo::select_summary(&state.pool, tenant_id, &filter).await?)
}

/// Lists stock movements.
pub async fn list_movements(
    state: &AppState,
    tenant_id: Uuid,
    filter: MovementFilter,
) -> Result<Page<StockMovement>, ApiError> {
    Ok(repo::select_movements(&state.pool, tenant_id, &filter).await?)
}

/// Number of (type, name, unit) groups with total amount below 1.0.
pub async fn count_low_stock(state: &AppState, tenant_id: Uuid) -> Result<i64, ApiError> {
    Ok(repo::count_low_stock(&state.pool, tenant_id).await?)
}

/// Builds a safe `ORDER BY` clause from a comma-separated sort spec.
fn build_ing_sort(sort: &str) -> Result<String, ApiError> {
    let spec = if sort.is_empty() {
        "best_before_date"
    } else {
        sort
    };
    let mut parts = Vec::new();
    for field in spec.split(',') {
        let field = field.trim();
        let desc = field.starts_with('-');
        let name = field.trim_start_matches('-');
        let col = match name {
            "best_before_date" => "best_before_date",
            "created_at" => "created_at",
            "name" => "name",
            "amount" => "amount",
            _ => {
                return Err(ApiError::validation(
                    "sort",
                    &format!("unknown sort field: {name}"),
                ))
            }
        };
        if col == "best_before_date" {
            parts.push(if desc {
                "best_before_date DESC NULLS FIRST".to_string()
            } else {
                "best_before_date ASC NULLS LAST".to_string()
            });
        } else {
            parts.push(format!("{col} {}", if desc { "DESC" } else { "ASC" }));
        }
    }
    Ok(parts.join(", "))
}
