//! Recipe business logic: CRUD with nested children, cached physics values,
//! and BeerXML/Brewfather import.
//!
//! Port of the Go `internal/recipe/service.go`.

use uuid::Uuid;

use super::calc;
use super::import_beerxml::parse_beerxml;
use super::import_brewfather::parse_brewfather;
use super::models::{
    CreateRequest, Fermentable, Hop, ListFilter, MashStep, Page, PatchRequest, Recipe,
    RecipeWithIngredients, Yeast,
};
use super::repository::{self as repo, RecipeWrite};
use crate::platform::errors::ApiError;
use crate::state::AppState;

fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .is_some_and(|d| d.is_unique_violation())
}

fn write_from_create(req: &CreateRequest) -> RecipeWrite {
    RecipeWrite {
        name: req.name.clone(),
        r#type: req.r#type.clone(),
        style_id: req.style_id,
        equipment_profile_id: req.equipment_profile_id,
        mash_profile_id: req.mash_profile_id,
        batch_size_liters: req.batch_size_liters,
        boil_size_liters: req.boil_size_liters,
        boil_time_minutes: req.boil_time_minutes,
        efficiency_pct: req.efficiency_pct,
        tasting_aroma: req.tasting_aroma.clone(),
        tasting_flavour: req.tasting_flavour.clone(),
        tasting_mouthfeel: req.tasting_mouthfeel.clone(),
        tasting_finish: req.tasting_finish.clone(),
        notes: req.notes.clone(),
    }
}

fn ferms_from_inputs(recipe_id: Uuid, req: &CreateRequest) -> Vec<Fermentable> {
    req.fermentables
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|i| Fermentable {
            id: Uuid::nil(),
            recipe_id,
            step_order: i.step_order,
            name: i.name.clone(),
            amount: i.amount,
            unit: i.unit.clone(),
            color_ebc: i.color_ebc,
            potential_ppg: i.potential_ppg,
            r#type: i.r#type.clone(),
            addition: i.addition.clone(),
            inventory_lot_id: None,
        })
        .collect()
}

fn hops_from_inputs(recipe_id: Uuid, req: &CreateRequest) -> Vec<Hop> {
    req.hops
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|i| Hop {
            id: Uuid::nil(),
            recipe_id,
            step_order: i.step_order,
            name: i.name.clone(),
            amount: i.amount,
            unit: i.unit.clone(),
            alpha_acid_pct: i.alpha_acid_pct,
            boil_time_minutes: i.boil_time_minutes,
            form: i.form.clone(),
            r#use: i.r#use.clone(),
            inventory_lot_id: None,
        })
        .collect()
}

fn yeasts_from_inputs(recipe_id: Uuid, req: &CreateRequest) -> Vec<Yeast> {
    req.yeasts
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|i| Yeast {
            id: Uuid::nil(),
            recipe_id,
            yeast_id: i.yeast_id,
            name: i.name.clone(),
            amount: i.amount,
            unit: i.unit.clone(),
            attenuation_pct: i.attenuation_pct,
            inventory_lot_id: None,
        })
        .collect()
}

fn steps_from_inputs(recipe_id: Uuid, req: &CreateRequest) -> Vec<MashStep> {
    req.mash_steps
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|i| MashStep {
            id: Uuid::nil(),
            recipe_id,
            step_order: i.step_order,
            step_type: i.step_type.clone(),
            target_temp_c: i.target_temp_c,
            hold_minutes: i.hold_minutes,
            infusion_volume_liters: i.infusion_volume_liters,
        })
        .collect()
}

/// Loads a recipe with all its children, returning 404 if absent.
async fn load(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<RecipeWithIngredients, ApiError> {
    let recipe = repo::select_row(&state.pool, tenant_id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("recipe"))?;
    Ok(RecipeWithIngredients {
        fermentables: repo::select_fermentables(&state.pool, id).await?,
        hops: repo::select_hops(&state.pool, id).await?,
        yeasts: repo::select_yeasts(&state.pool, id).await?,
        mash_steps: repo::select_mash_steps(&state.pool, id).await?,
        recipe,
    })
}

/// Creates a recipe, its children, and the cached calculations.
pub async fn create(
    state: &AppState,
    tenant_id: Uuid,
    req: CreateRequest,
) -> Result<RecipeWithIngredients, ApiError> {
    let mut tx = state.pool.begin().await?;
    let w = write_from_create(&req);
    let rec = match repo::insert(&mut *tx, tenant_id, &w).await {
        Ok(r) => r,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict(
                "recipe",
                "name already exists for this tenant",
            ));
        }
        Err(e) => return Err(e.into()),
    };

    let ferms = ferms_from_inputs(rec.id, &req);
    let hops = hops_from_inputs(rec.id, &req);
    let yeasts = yeasts_from_inputs(rec.id, &req);
    let steps = steps_from_inputs(rec.id, &req);

    repo::replace_fermentables(&mut tx, rec.id, &ferms).await?;
    repo::replace_hops(&mut tx, rec.id, &hops).await?;
    repo::replace_yeasts(&mut tx, rec.id, &yeasts).await?;
    repo::replace_mash_steps(&mut tx, rec.id, &steps).await?;

    let calcs = calc::compute_calcs(&rec, &ferms, &hops, &yeasts);
    repo::update_calculations(&mut *tx, rec.id, &calcs).await?;

    tx.commit().await?;
    load(state, tenant_id, rec.id).await
}

/// Lists recipes.
pub async fn list(
    state: &AppState,
    tenant_id: Uuid,
    filter: ListFilter,
) -> Result<Page<Recipe>, ApiError> {
    let order_by = build_sort(&filter.sort)?;
    Ok(repo::select_list(&state.pool, tenant_id, &filter, &order_by).await?)
}

/// Fetches a recipe with children.
pub async fn get(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
) -> Result<RecipeWithIngredients, ApiError> {
    load(state, tenant_id, id).await
}

/// Replaces a recipe (PUT). All four child arrays must be present.
pub async fn replace(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: CreateRequest,
) -> Result<RecipeWithIngredients, ApiError> {
    for (field, present) in [
        ("fermentables", req.fermentables.is_some()),
        ("hops", req.hops.is_some()),
        ("yeasts", req.yeasts.is_some()),
        ("mash_steps", req.mash_steps.is_some()),
    ] {
        if !present {
            return Err(ApiError::validation(
                field,
                "required for PUT; use [] to delete all",
            ));
        }
    }

    if repo::select_row(&state.pool, tenant_id, id)
        .await?
        .is_none()
    {
        return Err(ApiError::not_found("recipe"));
    }

    let mut tx = state.pool.begin().await?;
    let w = write_from_create(&req);
    let rec = match repo::update_full(&mut *tx, tenant_id, id, &w).await {
        Ok(Some(r)) => r,
        Ok(None) => return Err(ApiError::not_found("recipe")),
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict(
                "recipe",
                "name already exists for this tenant",
            ));
        }
        Err(e) => return Err(e.into()),
    };

    let ferms = ferms_from_inputs(rec.id, &req);
    let hops = hops_from_inputs(rec.id, &req);
    let yeasts = yeasts_from_inputs(rec.id, &req);
    let steps = steps_from_inputs(rec.id, &req);

    repo::replace_fermentables(&mut tx, rec.id, &ferms).await?;
    repo::replace_hops(&mut tx, rec.id, &hops).await?;
    repo::replace_yeasts(&mut tx, rec.id, &yeasts).await?;
    repo::replace_mash_steps(&mut tx, rec.id, &steps).await?;

    let calcs = calc::compute_calcs(&rec, &ferms, &hops, &yeasts);
    repo::update_calculations(&mut *tx, rec.id, &calcs).await?;

    tx.commit().await?;
    load(state, tenant_id, rec.id).await
}

/// Partially updates a recipe. Child arrays are replaced only when present.
pub async fn patch(
    state: &AppState,
    tenant_id: Uuid,
    id: Uuid,
    req: PatchRequest,
) -> Result<RecipeWithIngredients, ApiError> {
    let existing = load(state, tenant_id, id).await?;

    let mut tx = state.pool.begin().await?;

    let mut w = RecipeWrite {
        name: existing.recipe.name.clone(),
        r#type: existing.recipe.r#type.clone(),
        style_id: existing.recipe.style_id,
        equipment_profile_id: existing.recipe.equipment_profile_id,
        mash_profile_id: existing.recipe.mash_profile_id,
        batch_size_liters: existing.recipe.batch_size_liters,
        boil_size_liters: existing.recipe.boil_size_liters,
        boil_time_minutes: existing.recipe.boil_time_minutes,
        efficiency_pct: existing.recipe.efficiency_pct,
        tasting_aroma: existing.recipe.tasting_aroma.clone(),
        tasting_flavour: existing.recipe.tasting_flavour.clone(),
        tasting_mouthfeel: existing.recipe.tasting_mouthfeel.clone(),
        tasting_finish: existing.recipe.tasting_finish.clone(),
        notes: existing.recipe.notes.clone(),
    };
    if let Some(v) = req.name {
        w.name = v;
    }
    if let Some(v) = req.r#type {
        w.r#type = v;
    }
    if req.style_id.is_some() {
        w.style_id = req.style_id;
    }
    if req.equipment_profile_id.is_some() {
        w.equipment_profile_id = req.equipment_profile_id;
    }
    if req.mash_profile_id.is_some() {
        w.mash_profile_id = req.mash_profile_id;
    }
    if let Some(v) = req.batch_size_liters {
        w.batch_size_liters = v;
    }
    if req.boil_size_liters.is_some() {
        w.boil_size_liters = req.boil_size_liters;
    }
    if req.boil_time_minutes.is_some() {
        w.boil_time_minutes = req.boil_time_minutes;
    }
    if req.efficiency_pct.is_some() {
        w.efficiency_pct = req.efficiency_pct;
    }
    if req.tasting_aroma.is_some() {
        w.tasting_aroma = req.tasting_aroma;
    }
    if req.tasting_flavour.is_some() {
        w.tasting_flavour = req.tasting_flavour;
    }
    if req.tasting_mouthfeel.is_some() {
        w.tasting_mouthfeel = req.tasting_mouthfeel;
    }
    if req.tasting_finish.is_some() {
        w.tasting_finish = req.tasting_finish;
    }
    if req.notes.is_some() {
        w.notes = req.notes;
    }

    let rec = match repo::update_full(&mut *tx, tenant_id, id, &w).await {
        Ok(Some(r)) => r,
        Ok(None) => return Err(ApiError::not_found("recipe")),
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict(
                "recipe",
                "name already exists for this tenant",
            ));
        }
        Err(e) => return Err(e.into()),
    };

    // Start from current children; replace any array that was provided.
    let mut ferms = existing.fermentables;
    let mut hops = existing.hops;
    let mut yeasts = existing.yeasts;

    if let Some(inputs) = req.fermentables {
        ferms = build_children(rec.id, inputs, child_ferm);
        repo::replace_fermentables(&mut tx, rec.id, &ferms).await?;
    }
    if let Some(inputs) = req.hops {
        hops = build_children(rec.id, inputs, child_hop);
        repo::replace_hops(&mut tx, rec.id, &hops).await?;
    }
    if let Some(inputs) = req.yeasts {
        yeasts = build_children(rec.id, inputs, child_yeast);
        repo::replace_yeasts(&mut tx, rec.id, &yeasts).await?;
    }
    if let Some(inputs) = req.mash_steps {
        let steps = build_children(rec.id, inputs, child_step);
        repo::replace_mash_steps(&mut tx, rec.id, &steps).await?;
    }

    let calcs = calc::compute_calcs(&rec, &ferms, &hops, &yeasts);
    repo::update_calculations(&mut *tx, rec.id, &calcs).await?;

    tx.commit().await?;
    load(state, tenant_id, rec.id).await
}

/// Deletes a recipe unless a batch references it.
pub async fn delete(state: &AppState, tenant_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    if repo::select_row(&state.pool, tenant_id, id)
        .await?
        .is_none()
    {
        return Err(ApiError::not_found("recipe"));
    }
    if repo::is_referenced_by_batch(&state.pool, id).await? {
        return Err(ApiError::business_rule(
            "recipe_referenced_by_batch",
            "recipe is used by one or more batches",
            Default::default(),
        ));
    }
    let mut tx = state.pool.begin().await?;
    repo::delete_by_id(&mut *tx, tenant_id, id).await?;
    tx.commit().await?;
    Ok(())
}

/// Imports a recipe from BeerXML or Brewfather JSON. On name conflict, returns
/// the existing recipe.
pub async fn import(
    state: &AppState,
    tenant_id: Uuid,
    format: &str,
    data: &str,
) -> Result<RecipeWithIngredients, ApiError> {
    let req = match format {
        "beerxml" => parse_beerxml(data),
        "brewfather" => parse_brewfather(data),
        other => {
            return Err(ApiError::validation(
                "format",
                &format!("unknown format {other:?}"),
            ))
        }
    }
    .map_err(|e| ApiError::validation("data", &e))?;

    let name = req.name.clone();
    match create(state, tenant_id, req).await {
        Ok(rec) => Ok(rec),
        Err(e) if e.status() == axum::http::StatusCode::CONFLICT => {
            // Name already exists — return the existing recipe.
            let existing = repo::select_row_by_name(&state.pool, tenant_id, &name)
                .await?
                .ok_or(e)?;
            load(state, tenant_id, existing.id).await
        }
        Err(e) => Err(e),
    }
}

// Build domain children from validated inputs, generic over the per-type mapper.
fn build_children<I, T>(recipe_id: Uuid, inputs: Vec<I>, map: fn(Uuid, I) -> T) -> Vec<T> {
    inputs.into_iter().map(|i| map(recipe_id, i)).collect()
}

fn child_ferm(recipe_id: Uuid, i: super::models::FermentableInput) -> Fermentable {
    Fermentable {
        id: Uuid::nil(),
        recipe_id,
        step_order: i.step_order,
        name: i.name,
        amount: i.amount,
        unit: i.unit,
        color_ebc: i.color_ebc,
        potential_ppg: i.potential_ppg,
        r#type: i.r#type,
        addition: i.addition,
        inventory_lot_id: None,
    }
}

fn child_hop(recipe_id: Uuid, i: super::models::HopInput) -> Hop {
    Hop {
        id: Uuid::nil(),
        recipe_id,
        step_order: i.step_order,
        name: i.name,
        amount: i.amount,
        unit: i.unit,
        alpha_acid_pct: i.alpha_acid_pct,
        boil_time_minutes: i.boil_time_minutes,
        form: i.form,
        r#use: i.r#use,
        inventory_lot_id: None,
    }
}

fn child_yeast(recipe_id: Uuid, i: super::models::YeastInput) -> Yeast {
    Yeast {
        id: Uuid::nil(),
        recipe_id,
        yeast_id: i.yeast_id,
        name: i.name,
        amount: i.amount,
        unit: i.unit,
        attenuation_pct: i.attenuation_pct,
        inventory_lot_id: None,
    }
}

fn child_step(recipe_id: Uuid, i: super::models::MashStepInput) -> MashStep {
    MashStep {
        id: Uuid::nil(),
        recipe_id,
        step_order: i.step_order,
        step_type: i.step_type,
        target_temp_c: i.target_temp_c,
        hold_minutes: i.hold_minutes,
        infusion_volume_liters: i.infusion_volume_liters,
    }
}

/// Builds a safe `ORDER BY` from the sort spec (default `-created_at`).
fn build_sort(sort: &str) -> Result<String, ApiError> {
    let spec = if sort.is_empty() { "-created_at" } else { sort };
    let desc = spec.starts_with('-');
    let col = spec.trim_start_matches('-');
    let mapped = match col {
        "created_at" => "created_at",
        "updated_at" => "updated_at",
        "name" => "name",
        _ => "created_at",
    };
    Ok(format!("{mapped} {}", if desc { "DESC" } else { "ASC" }))
}
