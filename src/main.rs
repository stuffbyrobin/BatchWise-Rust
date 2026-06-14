//! Batchwise server entry point.
//!
//! Phase 0 (foundations): config + logger + database pool + migrations, an
//! axum app serving `GET /healthz`, and the request-id middleware. Domain
//! routers are mounted here in later phases.

use std::net::SocketAddr;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use ulid::Ulid;

use batchwise::platform::context::RequestContext;
use batchwise::platform::{config::Config, database, logger};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env for local development; ignore if absent.
    let _ = dotenvy::dotenv();

    let cfg = Config::load()?;
    logger::init(&cfg.app_env, &cfg.log_level);

    let pool = database::connect(&cfg.database_url).await?;
    if cfg.migrations_disabled {
        tracing::warn!("migrations disabled via MIGRATIONS_DISABLED");
    } else {
        database::migrate(&pool).await?;
        tracing::info!("migrations applied");
    }

    let app = build_router();

    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.http_port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "batchwise listening");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Builds the application router. Kept separate from `main` so tests can drive
/// it without binding a socket.
fn build_router() -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .layer(axum::middleware::from_fn(request_id_middleware))
}

/// Liveness probe.
async fn healthz() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

/// Generates a ULID request id, installs a [`RequestContext`] into the request
/// extensions, and echoes the id back as `X-Request-ID`.
async fn request_id_middleware(mut req: Request, next: Next) -> Response {
    let request_id = Ulid::new().to_string();
    req.extensions_mut().insert(RequestContext {
        request_id: request_id.clone(),
        ..Default::default()
    });
    let mut resp = next.run(req).await;
    if let Ok(value) = request_id.parse() {
        resp.headers_mut().insert("x-request-id", value);
    }
    resp
}
