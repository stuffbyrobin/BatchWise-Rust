//! Concurrency: requests that race each other must not double-count or
//! oversell. Each test fires the same request many times at once (the pool
//! allows 10 connections, so they genuinely overlap in the database) and checks
//! the resulting data, not just the status codes.

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
        trust_proxy_headers: false,
        redis_url: None,
        redis_key_prefix: "batchwise-test".into(),
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

/// Every feature flag, so both tenants can reach every gated route.
const ALL_FEATURES: &str = r#"{"inventory":true,"recipes":true,"batches":true,"calendar":true,"yeastkinetics":true,"library":true,"water":true,"yeast_banking":true,"fermentation":true,"tracking":true,"reporting":true,"sales":true,"duty":true,"allergens":true,"labels":true,"label_design":true,"packaging":true,"traceability":true,"equipment_maintenance":true,"procurement":true}"#;

impl TestApp {
    /// Registers a fresh user + tenant with every feature enabled; returns the token.
    async fn tenant(&self) -> String {
        let body = json!({
            "email": format!("iso-{}@example.com", uniq()),
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
        assert_eq!(resp.status(), 201, "register");
        let v: Value = resp.json().await.unwrap();
        let tenant_id = Uuid::parse_str(v["tenant_id"].as_str().unwrap()).unwrap();
        let pool = sqlx::PgPool::connect(&self.db_url).await.unwrap();
        sqlx::query("UPDATE tenants SET feature_flags = feature_flags || $2::jsonb WHERE id=$1")
            .bind(tenant_id)
            .bind(ALL_FEATURES)
            .execute(&pool)
            .await
            .unwrap();
        v["access_token"].as_str().unwrap().to_string()
    }

    async fn send(
        &self,
        token: &str,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> reqwest::Response {
        let mut req = self
            .client
            .request(method, format!("{}{path}", self.base))
            .bearer_auth(token);
        if let Some(b) = body {
            req = req.json(b);
        }
        req.send().await.unwrap()
    }

    /// POSTs as `token`, asserts 2xx, returns the JSON body.
    async fn create(&self, token: &str, path: &str, body: Value) -> Value {
        let resp = self
            .send(token, reqwest::Method::POST, path, Some(&body))
            .await;
        let status = resp.status();
        let text = resp.text().await.unwrap();
        assert!(status.is_success(), "create {path}: {status} {text}");
        serde_json::from_str(&text).unwrap()
    }

    /// GETs as `token`, asserts 200, returns the JSON body.
    async fn read(&self, token: &str, path: &str) -> Value {
        let resp = self.send(token, reqwest::Method::GET, path, None).await;
        let status = resp.status();
        let text = resp.text().await.unwrap();
        assert_eq!(status, 200, "read {path}: {text}");
        serde_json::from_str(&text).unwrap()
    }
}

fn id(v: &Value) -> String {
    v["id"].as_str().expect("id").to_string()
}

/// Sends `n` copies of the same request concurrently; returns the sorted statuses
/// (0 for a transport error).
async fn race(
    app: &TestApp,
    token: &str,
    method: reqwest::Method,
    path: &str,
    body: Value,
    n: usize,
) -> Vec<u16> {
    let mut set = tokio::task::JoinSet::new();
    for _ in 0..n {
        let client = app.client.clone();
        let url = format!("{}{path}", app.base);
        let token = token.to_string();
        let body = body.clone();
        let method = method.clone();
        set.spawn(async move {
            client
                .request(method, url)
                .bearer_auth(token)
                .json(&body)
                .send()
                .await
                .map(|r| r.status().as_u16())
                .unwrap_or(0)
        });
    }
    let mut statuses = Vec::with_capacity(n);
    while let Some(s) = set.join_next().await {
        statuses.push(s.unwrap());
    }
    statuses.sort_unstable();
    statuses
}

/// A batch with OG/FG set, so a sale of it crystallises a duty event.
async fn duty_batch(app: &TestApp, token: &str) -> String {
    let recipe = id(&app
        .create(
            token,
            "/api/v1/recipes",
            json!({"name": format!("R {}", uniq()), "type": "all_grain", "batch_size_liters": 100.0}),
        )
        .await);
    let batch_v = app
        .create(
            token,
            "/api/v1/batches",
            json!({"recipe_id": recipe, "batch_number": format!("B-{}", uniq()), "name": "Race batch"}),
        )
        .await;
    let batch = batch_v["batch"]["id"].as_str().unwrap().to_string();
    let resp = app
        .send(
            token,
            reqwest::Method::PATCH,
            &format!("/api/v1/batches/{batch}"),
            Some(&json!({"actual_og": 1.050, "actual_fg": 1.010})),
        )
        .await;
    assert_eq!(resp.status(), 200, "set OG/FG");
    batch
}

#[tokio::test]
async fn concurrent_fulfilment_crystallises_duty_once() {
    let app = spawn_app().await;
    let t = app.tenant().await;
    let batch = duty_batch(&app, &t).await;
    let customer = id(&app
        .create(
            &t,
            "/api/v1/customers",
            json!({"name": format!("C {}", uniq()), "country": "GB"}),
        )
        .await);
    let order = id(&app
        .create(&t, "/api/v1/orders", json!({"customer_id": customer}))
        .await);
    app.create(
        &t,
        &format!("/api/v1/orders/{order}/items"),
        json!({"batch_id": batch, "product_name": "Keg", "volume_liters": 50.0, "unit_price_pence": 10000, "quantity": 1}),
    )
    .await;
    let resp = app
        .send(
            &t,
            reqwest::Method::POST,
            &format!("/api/v1/orders/{order}/confirm"),
            Some(&json!({})),
        )
        .await;
    assert_eq!(resp.status(), 200, "confirm");

    let statuses = race(
        &app,
        &t,
        reqwest::Method::POST,
        &format!("/api/v1/orders/{order}/fulfill"),
        json!({}),
        8,
    )
    .await;
    assert_eq!(
        statuses.iter().filter(|s| **s == 200).count(),
        1,
        "exactly one fulfil wins: {statuses:?}"
    );
    assert!(
        statuses.iter().all(|s| *s == 200 || *s == 422),
        "the others get the FSM 422: {statuses:?}"
    );

    let events = app.read(&t, "/api/v1/duty-events?page_size=100").await;
    assert_eq!(
        events["total"],
        json!(1),
        "one duty event per item, not per request: {statuses:?} {events}"
    );
}

#[tokio::test]
async fn concurrent_sales_cannot_oversell_a_packaging_run() {
    let app = spawn_app().await;
    let t = app.tenant().await;
    let batch = duty_batch(&app, &t).await;
    let run = id(&app
        .create(
            &t,
            "/api/v1/packaging-runs",
            json!({"batch_id": batch, "format": "keg", "unit_volume_ml": 50000, "quantity": 10, "lot_number": format!("PKG-{}", uniq()), "packaged_at": "2026-07-01"}),
        )
        .await);

    let statuses = race(
        &app,
        &t,
        reqwest::Method::POST,
        "/api/v1/distribution-movements",
        json!({"packaging_run_id": run, "movement_type": "sample", "quantity": 1, "to_location": "Taproom"}),
        20,
    )
    .await;
    assert_eq!(
        statuses.iter().filter(|s| **s == 201).count(),
        10,
        "exactly the stock is sold: {statuses:?}"
    );
    assert!(
        statuses.iter().all(|s| *s == 201 || *s == 422),
        "the rest are insufficient_stock: {statuses:?}"
    );

    let run_v = app.read(&t, &format!("/api/v1/packaging-runs/{run}")).await;
    assert_eq!(run_v["stock_remaining"], json!(0), "{run_v}");
}

#[tokio::test]
async fn concurrent_batch_creates_with_one_number_leave_one_batch() {
    let app = spawn_app().await;
    let t = app.tenant().await;
    let recipe = id(&app
        .create(
            &t,
            "/api/v1/recipes",
            json!({"name": format!("R {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0}),
        )
        .await);
    let number = format!("RACE-{}", uniq());
    let body = json!({"recipe_id": recipe, "batch_number": number, "name": "Race"});
    // Seed a planned batch so every racer takes the replace-a-planned-batch path.
    app.create(&t, "/api/v1/batches", body.clone()).await;

    let statuses = race(&app, &t, reqwest::Method::POST, "/api/v1/batches", body, 8).await;
    assert!(
        statuses.iter().all(|s| *s == 201 || *s == 409),
        "no racer may fail with a server error: {statuses:?}"
    );

    let list = app.read(&t, "/api/v1/batches?page_size=100").await;
    let same = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["batch_number"] == json!(number))
        .count();
    assert_eq!(same, 1, "exactly one batch keeps the number: {statuses:?}");
}
