//! Sales HTTP handlers and routers (customers, orders, duty-events).
//!
//! Port of the Go `internal/sales` handlers. All routes require auth and the
//! `sales` feature flag (tier gate). Handlers only decode → validate → call the
//! service → render; business logic lives in [`super::service`].

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{
    CancelOrderRequest, CreateCustomerRequest, CreateItemRequest, CreateOrderRequest,
    CustomerFilter, DutyEventFilter, FulfillOrderRequest, OrderFilter, PatchCustomerRequest,
    PatchOrderRequest,
};
use super::service;
use crate::platform::context::RequestContext;
use crate::platform::errors::ApiError;
use crate::platform::middleware::{check_feature, require_auth};
use crate::platform::web::ValidatedJson;
use crate::state::AppState;

/// Builds the sales routers (customers, orders, duty-events), gated by auth +
/// the `sales` feature flag.
pub fn routes(state: AppState) -> Router {
    let customers = Router::new()
        .route("/", get(list_customers).post(create_customer))
        .route(
            "/{id}",
            get(get_customer)
                .put(replace_customer)
                .patch(patch_customer)
                .delete(delete_customer),
        );
    let orders = Router::new()
        .route("/", get(list_orders).post(create_order))
        .route(
            "/{id}",
            get(get_order).patch(patch_order).delete(delete_order),
        )
        .route("/{id}/confirm", post(confirm_order))
        .route("/{id}/fulfill", post(fulfill_order))
        .route("/{id}/invoice", post(invoice_order))
        .route("/{id}/cancel", post(cancel_order))
        .route("/{id}/items", post(add_item))
        .route(
            "/{id}/items/{item_id}",
            put(replace_item).delete(delete_item),
        );
    let duty_events = Router::new()
        .route("/", get(list_duty_events))
        .route("/{id}", get(get_duty_event));

    let st = state.clone();
    let feature_layer = axum::middleware::from_fn(move |req, next| {
        let st = st.clone();
        async move { check_feature(&st, "sales", req, next).await }
    });

    Router::new()
        .nest("/customers", customers)
        .nest("/orders", orders)
        .nest("/duty-events", duty_events)
        .route_layer(feature_layer)
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

// ---- query params ----

#[derive(Debug, Deserialize)]
struct CustomerQuery {
    q: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OrderQuery {
    customer_id: Option<Uuid>,
    status: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct DutyEventQuery {
    order_id: Option<Uuid>,
    batch_id: Option<Uuid>,
    from_date: Option<String>,
    to_date: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

fn page_or_default(page: Option<i64>) -> i64 {
    match page {
        Some(p) if p >= 1 => p,
        _ => 1,
    }
}

fn page_size_or_default(page_size: Option<i64>) -> i64 {
    match page_size {
        Some(p) if p >= 1 => p,
        _ => 20,
    }
}

// ---- customers ----

async fn create_customer(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateCustomerRequest>,
) -> Result<Response, ApiError> {
    let c = service::create_customer(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/customers/{}", c.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(c)).into_response())
}

async fn list_customers(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<CustomerQuery>,
) -> Result<Response, ApiError> {
    let filter = CustomerFilter {
        q: q.q,
        sort: q.sort.unwrap_or_default(),
        page: page_or_default(q.page),
        page_size: page_size_or_default(q.page_size),
    };
    Ok(Json(service::list_customers(&state, ctx.tenant_id()?, filter).await?).into_response())
}

async fn get_customer(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_customer(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn replace_customer(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CreateCustomerRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::replace_customer(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

async fn patch_customer(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchCustomerRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::patch_customer(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

async fn delete_customer(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_customer(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- orders ----

async fn create_order(
    State(state): State<AppState>,
    ctx: RequestContext,
    ValidatedJson(req): ValidatedJson<CreateOrderRequest>,
) -> Result<Response, ApiError> {
    let o = service::create_order(&state, ctx.tenant_id()?, req).await?;
    let location = format!("/api/v1/orders/{}", o.id);
    Ok((StatusCode::CREATED, [(header::LOCATION, location)], Json(o)).into_response())
}

async fn list_orders(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<OrderQuery>,
) -> Result<Response, ApiError> {
    let filter = OrderFilter {
        customer_id: q.customer_id,
        status: q.status,
        from_date: q.from_date,
        to_date: q.to_date,
        sort: q.sort.unwrap_or_default(),
        page: page_or_default(q.page),
        page_size: page_size_or_default(q.page_size),
    };
    Ok(Json(service::list_orders(&state, ctx.tenant_id()?, filter).await?).into_response())
}

async fn get_order(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_order(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn patch_order(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<PatchOrderRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::patch_order(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

async fn delete_order(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    service::delete_order(&state, ctx.tenant_id()?, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn confirm_order(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::confirm_order(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn fulfill_order(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<FulfillOrderRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::fulfill_order(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

async fn invoice_order(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::invoice_order(&state, ctx.tenant_id()?, id).await?).into_response())
}

async fn cancel_order(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CancelOrderRequest>,
) -> Result<Response, ApiError> {
    Ok(Json(service::cancel_order(&state, ctx.tenant_id()?, id, req).await?).into_response())
}

async fn add_item(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(order_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CreateItemRequest>,
) -> Result<Response, ApiError> {
    let item = service::add_item(&state, ctx.tenant_id()?, order_id, req).await?;
    let location = format!("/api/v1/orders/{order_id}/items/{}", item.id);
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(item),
    )
        .into_response())
}

async fn replace_item(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path((order_id, item_id)): Path<(Uuid, Uuid)>,
    ValidatedJson(req): ValidatedJson<CreateItemRequest>,
) -> Result<Response, ApiError> {
    Ok(
        Json(service::replace_item(&state, ctx.tenant_id()?, order_id, item_id, req).await?)
            .into_response(),
    )
}

async fn delete_item(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path((order_id, item_id)): Path<(Uuid, Uuid)>,
) -> Result<Response, ApiError> {
    service::delete_item(&state, ctx.tenant_id()?, order_id, item_id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---- duty events ----

async fn list_duty_events(
    State(state): State<AppState>,
    ctx: RequestContext,
    Query(q): Query<DutyEventQuery>,
) -> Result<Response, ApiError> {
    let filter = DutyEventFilter {
        order_id: q.order_id,
        batch_id: q.batch_id,
        from_date: q.from_date,
        to_date: q.to_date,
        sort: q.sort.unwrap_or_default(),
        page: page_or_default(q.page),
        page_size: page_size_or_default(q.page_size),
    };
    Ok(Json(service::list_duty_events(&state, ctx.tenant_id()?, filter).await?).into_response())
}

async fn get_duty_event(
    State(state): State<AppState>,
    ctx: RequestContext,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok(Json(service::get_duty_event(&state, ctx.tenant_id()?, id).await?).into_response())
}
