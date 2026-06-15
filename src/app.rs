//! Application router assembly.
//!
//! Builds the axum [`Router`] from [`AppState`] and installs the request-context
//! middleware. Kept in the library (not `main.rs`) so integration tests can
//! drive the full app without binding a socket.

use axum::body::{to_bytes, Body};
use axum::extract::Request;
use axum::http::header;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use ulid::Ulid;

use crate::platform::context::RequestContext;
use crate::state::AppState;
use crate::{
    allergens, audit, auth, batch, calendar, dashboard, duty, inventory, labels, library, openapi,
    packaging, procurement, recipe, reporting, sales, tenant, traceability, tracking, water,
    yeastbanking, yeastkinetics,
};

/// Builds the full application router, mounting auth and tenant under `/api/v1`.
pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .nest("/auth", auth::routes(state.clone()))
        .nest("/tenants", tenant::routes(state.clone()))
        .nest("/inventory", inventory::routes(state.clone()))
        .nest("/library", library::routes(state.clone()))
        .nest(
            "/recipes",
            recipe::routes(state.clone()).merge(allergens::routes(state.clone())),
        )
        .nest("/batches", batch::routes(state.clone()))
        .nest("/calendar-events", calendar::routes(state.clone()))
        .nest("/yeast-kinetics", yeastkinetics::routes(state.clone()))
        .nest("/reporting", reporting::routes(state.clone()))
        .nest("/dashboard", dashboard::routes(state.clone()))
        .nest("/duty-returns", duty::routes(state.clone()))
        .nest("/label-records", labels::routes(state.clone()))
        .merge(openapi::routes())
        .merge(tracking::routes(state.clone()))
        .merge(sales::routes(state.clone()))
        .merge(water::routes(state.clone()))
        .merge(packaging::routes(state.clone()))
        .merge(procurement::routes(state.clone()))
        .merge(yeastbanking::routes(state.clone()))
        .merge(traceability::routes(state.clone()))
        .merge(audit::routes(state.clone()));

    Router::new()
        .route("/healthz", get(healthz))
        .nest("/api/v1", api)
        .layer(axum::middleware::from_fn(request_context_middleware))
}

/// Liveness probe.
async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

/// Generates a ULID request id, installs a [`RequestContext`], echoes the id as
/// `X-Request-ID`, and stamps it into JSON error bodies (which carry an empty
/// `request_id` placeholder until this layer fills it in).
async fn request_context_middleware(mut req: Request, next: Next) -> Response {
    let request_id = Ulid::new().to_string();
    req.extensions_mut().insert(RequestContext {
        request_id: request_id.clone(),
        ..Default::default()
    });

    let resp = next.run(req).await;
    let (mut parts, body) = resp.into_parts();
    if let Ok(value) = request_id.parse() {
        parts.headers.insert("x-request-id", value);
    }

    let is_error = parts.status.is_client_error() || parts.status.is_server_error();
    let is_json = parts
        .headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|c| c.starts_with("application/json"));

    if is_error && is_json {
        let bytes = to_bytes(body, 64 * 1024).await.unwrap_or_default();
        if let Ok(mut v) = serde_json::from_slice::<Value>(&bytes) {
            if v.get("request_id").and_then(Value::as_str) == Some("") {
                v["request_id"] = json!(request_id);
            }
            return Response::from_parts(parts, Body::from(v.to_string()));
        }
        return Response::from_parts(parts, Body::from(bytes));
    }

    Response::from_parts(parts, body)
}
