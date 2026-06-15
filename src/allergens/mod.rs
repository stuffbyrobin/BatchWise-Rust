//! Recipe allergen declaration: name-matches a recipe's ingredients against
//! inventory-lot allergen arrays (tenant + system lots) and unions the result.
//!
//! Port of the Go `internal/compliance/allergens` package. Mounted at
//! `GET /recipes/{id}/allergens`, gated by the `allergens` feature flag. The
//! Go service's fire-and-forget audit write is omitted (no audit module yet).

use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use uuid::Uuid;

use crate::pkg::allergen;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{check_feature, require_auth};
use crate::state::AppState;

const SYSTEM_TENANT_ID: Uuid = Uuid::nil();

/// Computed allergen set for a recipe.
#[derive(Debug, Serialize)]
pub struct AllergenResult {
    pub recipe_id: Uuid,
    pub allergens: Vec<String>,
    pub ingredient_names: Vec<String>,
    pub unmatched: Vec<String>,
}

/// Router providing `GET /{id}/allergens`, merged into the recipe nest by the
/// orchestrator so the full path is `/recipes/{id}/allergens`.
pub fn routes(state: AppState) -> Router {
    let st = state.clone();
    let feature_layer = axum::middleware::from_fn(move |req, next| {
        let st = st.clone();
        async move { check_feature(&st, "allergens", req, next).await }
    });
    Router::new()
        .route("/{id}/allergens", get(compute))
        .route_layer(feature_layer)
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

async fn compute(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(recipe_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let tenant_id = ctx.tenant_id()?;
    Ok(Json(compute_for_recipe(&state, tenant_id, recipe_id).await?).into_response())
}

/// Computes the allergen declaration for a recipe.
pub async fn compute_for_recipe(
    state: &AppState,
    tenant_id: Uuid,
    recipe_id: Uuid,
) -> Result<AllergenResult, ApiError> {
    let names = ingredient_names_by_recipe(state, tenant_id, recipe_id).await?;
    let allergen_map =
        allergens_by_ingredient_names(state, &[tenant_id, SYSTEM_TENANT_ID], &names).await?;

    let mut combined: Vec<String> = Vec::new();
    let mut matched: Vec<String> = Vec::new();
    let mut unmatched: Vec<String> = Vec::new();

    for name in &names {
        let key = name.to_lowercase();
        if let Some(tokens) = allergen_map.get(&key) {
            combined = allergen::union(&combined, tokens);
            matched.push(name.clone());
        } else {
            unmatched.push(name.clone());
        }
    }

    Ok(AllergenResult {
        recipe_id,
        allergens: combined,
        ingredient_names: dedup_sorted(matched),
        unmatched: dedup_sorted(unmatched),
    })
}

/// Case-insensitively de-duplicates and sorts a list of names.
fn dedup_sorted(names: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<String> = names
        .into_iter()
        .filter(|n| seen.insert(n.to_lowercase()))
        .collect();
    out.sort();
    out
}

/// Returns all ingredient names (fermentables, hops, yeasts) for a recipe,
/// verifying the recipe belongs to the tenant.
async fn ingredient_names_by_recipe(
    state: &AppState,
    tenant_id: Uuid,
    recipe_id: Uuid,
) -> Result<Vec<String>, ApiError> {
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT tenant_id FROM recipes WHERE id = $1")
        .bind(recipe_id)
        .fetch_optional(&state.pool)
        .await?;
    match owner {
        Some(t) if t == tenant_id => {}
        _ => return Err(ApiError::not_found("recipe")),
    }

    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM recipe_fermentables WHERE recipe_id = $1 \
         UNION ALL SELECT name FROM recipe_hops WHERE recipe_id = $1 \
         UNION ALL SELECT name FROM recipe_yeasts WHERE recipe_id = $1",
    )
    .bind(recipe_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(names)
}

/// Maps lowercased ingredient name → allergen tokens for matching inventory
/// lots across the given tenants (current + system).
async fn allergens_by_ingredient_names(
    state: &AppState,
    tenant_ids: &[Uuid],
    names: &[String],
) -> Result<HashMap<String, Vec<String>>, ApiError> {
    if names.is_empty() {
        return Ok(HashMap::new());
    }
    let lowered: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
    let rows: Vec<(String, Vec<String>)> = sqlx::query_as(
        "SELECT lower(name), allergens FROM ingredients \
         WHERE tenant_id = ANY($1) AND allergens IS NOT NULL \
         AND array_length(allergens, 1) > 0 AND lower(name) = ANY($2)",
    )
    .bind(tenant_ids)
    .bind(&lowered)
    .fetch_all(&state.pool)
    .await?;
    Ok(rows.into_iter().collect())
}
