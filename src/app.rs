//! Application router assembly.
//!
//! Builds the axum [`Router`] from [`AppState`] and installs the middleware
//! stack: request id, request tracing, security headers (`nosniff`,
//! `X-Frame-Options: DENY`, `Referrer-Policy`, HSTS outside development), CORS
//! from `CORS_ORIGIN`, a 30 s timeout, a 2 MiB default body cap, and a global
//! per-IP rate limit on `/api/v1`. Kept in the library (not `main.rs`) so
//! integration tests can drive the full app without binding a socket.

use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::extract::{DefaultBodyLimit, Request};
use axum::http::{header, HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware::{from_fn_with_state, Next};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;
use ulid::Ulid;

use crate::platform::config::Config;
use crate::platform::context::RequestContext;
use crate::platform::middleware::{rate_limit, RateLimit};
use crate::state::AppState;
use crate::{
    allergens, audit, auth, batch, calendar, dashboard, duty, equipment, fermentation, fermenter,
    inventory, labeldesign, labels, library, openapi, packaging, procurement, recipe, reporting,
    sales, tenant, traceability, tracking, water, yeastbanking, yeastkinetics,
};

/// Wall-clock budget for one request; slower requests get 503.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Default request-body cap for every extractor (JSON, multipart). Routes that
/// need more (brand-asset upload) override it with their own `DefaultBodyLimit`.
pub const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
/// HSTS policy sent outside development.
const HSTS: &str = "max-age=31536000";

/// Builds the full application router, mounting auth and tenant under `/api/v1`.
pub fn build_router(state: AppState) -> Router {
    let cfg = state.config.clone();

    let api = Router::new()
        .nest("/auth", auth::routes(state.clone()))
        .nest("/tenants", tenant::routes(state.clone()))
        .nest("/inventory", inventory::routes(state.clone()))
        .nest("/library", library::routes(state.clone()))
        .nest(
            "/recipes",
            recipe::routes(state.clone()).merge(allergens::routes(state.clone())),
        )
        .nest(
            "/batches",
            batch::routes(state.clone()).merge(fermentation::routes(state.clone())),
        )
        .nest("/calendar-events", calendar::routes(state.clone()))
        .nest("/fermenters", fermenter::routes(state.clone()))
        .nest("/yeast-kinetics", yeastkinetics::routes(state.clone()))
        .merge(reporting::routes(state.clone()))
        .nest("/dashboard", dashboard::routes(state.clone()))
        .nest("/duty-returns", duty::routes(state.clone()))
        .nest("/label-records", labels::routes(state.clone()))
        .merge(openapi::routes())
        .merge(tracking::routes(state.clone()))
        .merge(sales::routes(state.clone()))
        .merge(water::routes(state.clone()))
        .merge(packaging::routes(state.clone()))
        .merge(procurement::routes(state.clone()))
        .merge(equipment::routes(state.clone()))
        .merge(labeldesign::routes(state.clone()))
        .merge(yeastbanking::routes(state.clone()))
        .merge(traceability::routes(state.clone()))
        .merge(audit::routes(state.clone()));

    // Global per-IP limit on the API only (not /healthz). CORS preflights are
    // answered by the CORS layer further out and never reach it.
    let api = api.layer(from_fn_with_state(
        RateLimit::per_minute(cfg.rate_limit_default_per_minute, cfg.trust_proxy_headers),
        rate_limit,
    ));
    let hsts = (!cfg.is_development()).then(|| HeaderValue::from_static(HSTS));

    // Outermost last: request id -> trace -> security headers -> CORS -> timeout -> body limit.
    Router::new()
        .route("/healthz", get(healthz))
        .nest("/api/v1", api)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            REQUEST_TIMEOUT,
        ))
        .layer(cors_layer(&cfg))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::STRICT_TRANSPORT_SECURITY,
            hsts,
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request| {
                    let request_id = req
                        .extensions()
                        .get::<RequestContext>()
                        .map(|c| c.request_id.as_str())
                        .unwrap_or("");
                    // Path only: query strings can carry search terms.
                    tracing::info_span!(
                        "http",
                        method = %req.method(),
                        path = %req.uri().path(),
                        request_id,
                    )
                })
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(axum::middleware::from_fn(request_context_middleware))
}

/// CORS for browser clients on other origins. Credentials are never allowed:
/// auth is a bearer token, not a cookie.
fn cors_layer(cfg: &Config) -> CorsLayer {
    let origins = cfg.cors_origins();
    let allow_origin = if origins.contains(&"*") {
        AllowOrigin::any()
    } else {
        AllowOrigin::list(origins.iter().filter_map(|o| o.parse::<HeaderValue>().ok()))
    };
    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
        .expose_headers([
            HeaderName::from_static("x-request-id"),
            header::RETRY_AFTER,
            header::LOCATION,
        ])
        .max_age(Duration::from_secs(3600))
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    fn config(app_env: &str, default_per_minute: u32) -> Config {
        Config {
            app_env: app_env.into(),
            app_base_url: "http://localhost:8080".into(),
            http_port: 8080,
            database_url: "postgres://unused@127.0.0.1:1/unused".into(),
            jwt_secret: "test-secret-key-at-least-32-bytes-long!!".into(),
            jwt_issuer: "batchwise".into(),
            jwt_audience: "batchwise".into(),
            jwt_expiry_minutes: 15,
            refresh_token_expiry_days: 7,
            cors_origin: "http://localhost:5173".into(),
            allow_overdraft: false,
            bootstrap_registration_enabled: false,
            rate_limit_register_per_minute: 1000,
            rate_limit_login_per_minute: 1000,
            rate_limit_refresh_per_minute: 1000,
            rate_limit_default_per_minute: default_per_minute,
            trust_proxy_headers: false,
            migrations_disabled: true,
            log_level: "info".into(),
        }
    }

    /// Full router over a lazy pool. None of these tests reach the database.
    fn app(cfg: Config) -> Router {
        let pool = PgPoolOptions::new()
            .connect_lazy(&cfg.database_url)
            .expect("lazy pool");
        build_router(AppState::new(pool, cfg))
    }

    fn get_req(uri: &str) -> Request {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    fn preflight(origin: &str) -> Request {
        Request::builder()
            .method("OPTIONS")
            .uri("/api/v1/auth/login")
            .header(header::ORIGIN, origin)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .header(
                header::ACCESS_CONTROL_REQUEST_HEADERS,
                "authorization,content-type",
            )
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn security_headers_on_every_response() {
        let resp = app(config("test", 1000))
            .oneshot(get_req("/healthz"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let h = resp.headers();
        assert_eq!(h[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(h[header::X_FRAME_OPTIONS], "DENY");
        assert_eq!(h[header::REFERRER_POLICY], "no-referrer");
        assert!(h.contains_key("x-request-id"));
        assert!(!h.contains_key(header::STRICT_TRANSPORT_SECURITY));
    }

    #[tokio::test]
    async fn hsts_outside_development() {
        let resp = app(config("production", 1000))
            .oneshot(get_req("/healthz"))
            .await
            .unwrap();
        assert_eq!(resp.headers()[header::STRICT_TRANSPORT_SECURITY], HSTS);
    }

    #[tokio::test]
    async fn cors_preflight_allows_configured_origin_without_credentials() {
        let resp = app(config("test", 1000))
            .oneshot(preflight("http://localhost:5173"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let h = resp.headers();
        assert_eq!(
            h[header::ACCESS_CONTROL_ALLOW_ORIGIN],
            "http://localhost:5173"
        );
        assert!(!h.contains_key(header::ACCESS_CONTROL_ALLOW_CREDENTIALS));
    }

    #[tokio::test]
    async fn cors_ignores_unknown_origin() {
        let resp = app(config("test", 1000))
            .oneshot(preflight("https://evil.example"))
            .await
            .unwrap();
        assert!(!resp
            .headers()
            .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
    }

    #[tokio::test]
    async fn global_rate_limit_covers_api_but_not_healthz() {
        let app = app(config("test", 2));
        for _ in 0..2 {
            let resp = app
                .clone()
                .oneshot(get_req("/api/v1/openapi.yaml"))
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        }
        let resp = app
            .clone()
            .oneshot(get_req("/api/v1/openapi.yaml"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(resp.headers().contains_key(header::RETRY_AFTER));

        let resp = app.oneshot(get_req("/healthz")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn oversized_json_body_is_413() {
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(vec![b' '; MAX_BODY_BYTES + 1]))
            .unwrap();
        let resp = app(config("test", 1000)).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn swagger_ui_assets_are_pinned_with_sri() {
        let resp = app(config("test", 1000))
            .oneshot(get_req("/api/v1/docs"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(!html.contains("swagger-ui-dist@5/"));
        assert_eq!(html.matches("swagger-ui-dist@5.32.15/").count(), 2);
        assert_eq!(html.matches("integrity=\"sha384-").count(), 2);
    }

    /// `(METHOD, path)` for every operation in openapi.yaml.
    fn openapi_operations() -> Vec<(String, String)> {
        let spec = include_str!("../openapi.yaml");
        let mut operations = Vec::new();
        let mut current: Option<String> = None;
        for line in spec.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            if indent == 0 {
                current = None;
            } else if indent == 2 {
                current = trimmed
                    .strip_suffix(':')
                    .filter(|p| p.starts_with("/api/v1/"))
                    .map(str::to_string);
            } else if indent == 4 {
                if let (Some(path), Some(method)) = (&current, trimmed.strip_suffix(':')) {
                    if ["get", "post", "put", "patch", "delete"].contains(&method) {
                        operations.push((method.to_uppercase(), path.clone()));
                    }
                }
            }
        }
        assert!(
            operations.len() > 100,
            "parsed only {} operations",
            operations.len()
        );

        operations
    }

    /// Every authenticated operation has a permission rule. Routes without one
    /// are refused for every role, so a missing rule would lock everyone out.
    #[test]
    fn every_openapi_operation_has_a_permission_rule() {
        const PUBLIC: [&str; 3] = [
            "/api/v1/auth/register",
            "/api/v1/auth/login",
            "/api/v1/auth/refresh",
        ];
        let missing: Vec<String> = openapi_operations()
            .into_iter()
            .filter(|(_, path)| !PUBLIC.contains(&path.as_str()))
            .filter(|(method, path)| {
                let method = Method::from_bytes(method.as_bytes()).expect("http method");
                crate::platform::authz::requirement(&method, path).is_none()
            })
            .map(|(method, path)| format!("{method} {path}"))
            .collect();
        assert!(
            missing.is_empty(),
            "operations without a permission rule:\n{}",
            missing.join("\n")
        );
    }

    /// Every operation in openapi.yaml must reach a route: an unrouted request is
    /// a 404 (unknown path) or 405 (unknown method). Requests carry no token and
    /// no body, so they stop at auth or body extraction and never touch the
    /// database. Guards against the router drifting from the published contract.
    #[tokio::test]
    async fn every_openapi_operation_is_routed() {
        let operations = openapi_operations();

        let app = app(config("test", 100_000));
        let mut unrouted = Vec::new();
        for (method, path) in &operations {
            // Replace every `{param}` with a UUID; routing does not check types.
            let mut uri = String::new();
            let mut rest = path.as_str();
            while let Some(open) = rest.find('{') {
                let close = rest[open..].find('}').expect("closing brace") + open;
                uri.push_str(&rest[..open]);
                uri.push_str("00000000-0000-0000-0000-000000000001");
                rest = &rest[close + 1..];
            }
            uri.push_str(rest);

            let req = Request::builder()
                .method(method.as_str())
                .uri(&uri)
                .body(Body::empty())
                .unwrap();
            let status = app.clone().oneshot(req).await.unwrap().status();
            if status == StatusCode::NOT_FOUND || status == StatusCode::METHOD_NOT_ALLOWED {
                unrouted.push(format!("{method} {path} -> {status}"));
            }
        }
        assert!(
            unrouted.is_empty(),
            "operations in openapi.yaml with no route:\n{}",
            unrouted.join("\n")
        );
    }
}
