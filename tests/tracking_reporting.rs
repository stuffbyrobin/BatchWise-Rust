//! Integration tests for the tier-gated tracking and reporting modules:
//! the feature-flag gate (home tenant blocked), container asset lifecycle,
//! QR-code generation, and reporting cost-rate CRUD.

use std::net::SocketAddr;

use batchwise::platform::config::Config;
use batchwise::platform::database;
use batchwise::state::AppState;
use serde_json::{json, Value};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

struct TestApp {
    base: String,
    db_url: String,
    client: reqwest::Client,
    _node: Option<ContainerAsync<Postgres>>,
}

fn uniq() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn test_config(database_url: String) -> Config {
    Config {
        app_env: "test".into(),
        app_base_url: "http://localhost:8080".into(),
        http_port: 0,
        database_url,
        jwt_secret: "test-secret-key-at-least-32-bytes-long!!".into(),
        jwt_issuer: "batchwise".into(),
        jwt_audience: "batchwise".into(),
        jwt_expiry_minutes: 15,
        refresh_token_expiry_days: 7,
        cors_origin: "http://localhost:5173".into(),
        allow_overdraft: false,
        bootstrap_registration_enabled: true,
        rate_limit_register_per_minute: 1000,
        rate_limit_login_per_minute: 1000,
        rate_limit_refresh_per_minute: 1000,
        rate_limit_default_per_minute: 1000,
        migrations_disabled: false,
        log_level: "info".into(),
    }
}

async fn spawn_app() -> TestApp {
    let (url, node) = match std::env::var("TEST_DATABASE_URL") {
        Ok(url) => (url, None),
        Err(_) => {
            let node = Postgres::default()
                .with_tag("16-alpine")
                .start()
                .await
                .expect("pg");
            let port = node.get_host_port_ipv4(5432).await.expect("port");
            (
                format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres"),
                Some(node),
            )
        }
    };
    let pool = database::connect(&url).await.expect("connect");
    database::migrate(&pool).await.expect("migrate");
    let state = AppState::new(pool, test_config(url.clone()));
    let app = batchwise::app::build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    TestApp {
        base: format!("http://{addr}"),
        db_url: url,
        client: reqwest::Client::new(),
        _node: node,
    }
}

impl TestApp {
    /// Registers a fresh owner; returns (token, tenant_id).
    async fn register(&self) -> (String, Uuid) {
        let body = json!({
            "email": format!("t-{}@example.com", uniq()),
            "password": "Sup3rSecret!pw",
            "display_name": "Tester",
            "tenant_name": format!("Brewery {}", uniq()),
        });
        let resp = self
            .client
            .post(format!("{}/api/v1/auth/register", self.base))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201);
        let v: Value = resp.json().await.unwrap();
        let token = v["access_token"].as_str().unwrap().to_string();
        let tenant_id = Uuid::parse_str(v["tenant_id"].as_str().unwrap()).unwrap();
        (token, tenant_id)
    }

    /// Enables the pro-tier feature flags directly in the DB (no API to upgrade tier).
    async fn enable_pro(&self, tenant_id: Uuid) {
        let pool = sqlx::PgPool::connect(&self.db_url).await.unwrap();
        sqlx::query(
            "UPDATE tenants SET feature_flags = feature_flags || '{\"tracking\":true,\"reporting\":true}'::jsonb WHERE id=$1",
        )
        .bind(tenant_id)
        .execute(&pool)
        .await
        .unwrap();
    }

    async fn post(&self, path: &str, token: &str, body: Value) -> reqwest::Response {
        self.client
            .post(format!("{}{path}", self.base))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .unwrap()
    }

    async fn get(&self, path: &str, token: &str) -> reqwest::Response {
        self.client
            .get(format!("{}{path}", self.base))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn tier_gate_blocks_home_tenant() {
    let app = spawn_app().await;
    let (token, _tid) = app.register().await;

    // home tier lacks the tracking + reporting feature flags → 403.
    let resp = app.get("/api/v1/container-assets", &token).await;
    assert_eq!(resp.status(), 403);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["code"], json!("forbidden"));
    assert_eq!(body["details"]["required_feature"], json!("tracking"));
    assert_eq!(body["details"]["current_tier"], json!("home"));

    let resp = app.get("/api/v1/reporting/cost-rates", &token).await;
    assert_eq!(resp.status(), 403);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["details"]["required_feature"],
        json!("reporting")
    );
}

#[tokio::test]
async fn container_asset_lifecycle() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable_pro(tid).await;

    let asset: Value = app
        .post("/api/v1/container-assets", &token, json!({"asset_number": format!("K-{}", uniq()), "container_type": "keg", "capacity_liters": 50.0}))
        .await
        .json()
        .await
        .unwrap();
    let id = asset["id"].as_str().unwrap();
    assert_eq!(asset["status"], json!("empty"));

    // Cannot deliver an empty container.
    let resp = app
        .post(
            &format!("/api/v1/container-assets/{id}/deliver"),
            &token,
            json!({"customer_name": "Pub"}),
        )
        .await;
    assert_eq!(resp.status(), 422);

    // empty -> filled -> delivered -> returned(empty).
    let filled: Value = app
        .post(
            &format!("/api/v1/container-assets/{id}/fill"),
            &token,
            json!({}),
        )
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(filled["status"], json!("filled"));
    let delivered: Value = app
        .post(
            &format!("/api/v1/container-assets/{id}/deliver"),
            &token,
            json!({"customer_name": "The Pub"}),
        )
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(delivered["status"], json!("delivered"));
    assert_eq!(delivered["current_customer_name"], json!("The Pub"));
    let returned: Value = app
        .post(
            &format!("/api/v1/container-assets/{id}/return"),
            &token,
            json!({}),
        )
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(returned["status"], json!("empty"));

    // A log trail exists.
    let logs: Value = app
        .get(&format!("/api/v1/container-logs?container_id={id}"), &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(logs["total"].as_i64().unwrap() >= 3);
}

#[tokio::test]
async fn qr_generation_json_and_png() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable_pro(tid).await;

    let asset: Value = app
        .post("/api/v1/container-assets", &token, json!({"asset_number": format!("K-{}", uniq()), "container_type": "keg", "capacity_liters": 30.0}))
        .await
        .json()
        .await
        .unwrap();
    let id = asset["id"].as_str().unwrap();

    // Variant A as JSON.
    let resp = app
        .client
        .get(format!("{}/api/v1/qr-codes/{id}/a", app.base))
        .bearer_auth(&token)
        .header("accept", "application/json")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["variant"], json!("a"));
    assert!(body["payload"].as_str().unwrap().contains(id));
    assert!(!body["png_base64"].as_str().unwrap().is_empty());

    // Variant B as raw PNG.
    let resp = app
        .client
        .get(format!("{}/api/v1/qr-codes/{id}/b", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()["content-type"], "image/png");
    let bytes = resp.bytes().await.unwrap();
    assert_eq!(
        &bytes[..8],
        &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
    );
}

#[tokio::test]
async fn reporting_cost_rate_crud() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable_pro(tid).await;

    let resp = app
        .post("/api/v1/reporting/cost-rates", &token, json!({"rate_type": "energy", "rate_name": "Electricity", "unit": "pence_per_kwh", "rate_value": 30.0, "effective_from": "2026-01-01"}))
        .await;
    assert_eq!(resp.status(), 201, "create rate");
    let rate: Value = resp.json().await.unwrap();
    let id = rate["id"].as_str().unwrap();
    assert_eq!(rate["rate_type"], json!("energy"));

    assert_eq!(
        app.get(&format!("/api/v1/reporting/cost-rates/{id}"), &token)
            .await
            .status(),
        200
    );
    let page: Value = app
        .get("/api/v1/reporting/cost-rates", &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(page["total"].as_i64().unwrap() >= 1);
}
