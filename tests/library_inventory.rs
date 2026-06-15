//! Integration tests for the library and inventory modules against real
//! Postgres. Covers the required Phase 2 scenarios: seeded reference data via
//! the library union-read, inventory CRUD, and FIFO-by-best-before-date
//! deduction (ordering, tie-breaks, overdraft on/off).

use std::net::SocketAddr;

use batchwise::platform::config::Config;
use batchwise::platform::{database, seed};
use batchwise::state::AppState;
use serde_json::{json, Value};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

struct TestApp {
    base: String,
    client: reqwest::Client,
    _node: Option<ContainerAsync<Postgres>>,
}

fn uniq() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn test_config(database_url: String, allow_overdraft: bool) -> Config {
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
        allow_overdraft,
        bootstrap_registration_enabled: true,
        rate_limit_register_per_minute: 1000,
        rate_limit_login_per_minute: 1000,
        rate_limit_refresh_per_minute: 1000,
        rate_limit_default_per_minute: 1000,
        migrations_disabled: false,
        log_level: "info".into(),
    }
}

async fn spawn_app(allow_overdraft: bool) -> TestApp {
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
    seed::run(&pool).await.expect("seed");

    let state = AppState::new(pool, test_config(url, allow_overdraft));
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
        client: reqwest::Client::new(),
        _node: node,
    }
}

impl TestApp {
    /// Registers a fresh owner and returns the bearer access token.
    async fn token(&self) -> String {
        let body = json!({
            "email": format!("u-{}@example.com", uniq()),
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
        resp.json::<Value>().await.unwrap()["access_token"]
            .as_str()
            .unwrap()
            .to_string()
    }

    async fn create_lot(
        &self,
        token: &str,
        name: &str,
        amount: f64,
        lot_number: &str,
        bbd: Option<&str>,
    ) -> Value {
        let mut body = json!({
            "type": "hop",
            "name": name,
            "amount": amount,
            "unit": "kg",
            "lot_number": lot_number,
        });
        if let Some(d) = bbd {
            body["best_before_date"] = json!(d);
        }
        let resp = self
            .client
            .post(format!("{}/api/v1/inventory", self.base))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201, "create lot should 201");
        resp.json().await.unwrap()
    }

    async fn deduct(&self, token: &str, name: &str, amount: f64) -> reqwest::Response {
        self.client
            .post(format!("{}/api/v1/inventory/deduct", self.base))
            .bearer_auth(token)
            .json(&json!({"type": "hop", "name": name, "amount": amount, "unit": "kg"}))
            .send()
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn library_returns_seeded_system_styles_and_tenant_union() {
    let app = spawn_app(false).await;
    let token = app.token().await;

    // Seeded system styles are visible to any tenant.
    let resp = app
        .client
        .get(format!("{}/api/v1/library/styles", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let page: Value = resp.json().await.unwrap();
    let seeded_total = page["total"].as_i64().unwrap();
    assert!(
        seeded_total > 0,
        "expected seeded styles, got {seeded_total}"
    );

    // Creating a tenant-owned style adds to the union.
    let name = format!("House IPA {}", uniq());
    let resp = app
        .client
        .post(format!("{}/api/v1/library/styles", app.base))
        .bearer_auth(&token)
        .json(&json!({"name": name}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = app
        .client
        .get(format!("{}/api/v1/library/styles", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let page: Value = resp.json().await.unwrap();
    assert_eq!(page["total"].as_i64().unwrap(), seeded_total + 1);
}

#[tokio::test]
async fn inventory_create_get_and_list() {
    let app = spawn_app(false).await;
    let token = app.token().await;
    let name = format!("Citra {}", uniq());

    let lot = app
        .create_lot(
            &token,
            &name,
            5.0,
            &format!("LOT-{}", uniq()),
            Some("2026-09-01"),
        )
        .await;
    assert_eq!(lot["amount"].as_f64().unwrap(), 5.0);
    let id = lot["id"].as_str().unwrap();

    let resp = app
        .client
        .get(format!("{}/api/v1/inventory/{id}", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = app
        .client
        .get(format!("{}/api/v1/inventory?name={name}", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let page: Value = resp.json().await.unwrap();
    assert_eq!(page["total"].as_i64().unwrap(), 1);
}

#[tokio::test]
async fn fifo_consumes_by_best_before_date_then_nulls_last() {
    let app = spawn_app(false).await;
    let token = app.token().await;
    let name = format!("Fuggle {}", uniq());

    // Three lots: dated 2026-12-01, dated 2026-08-01, and undated (null).
    app.create_lot(
        &token,
        &name,
        2.0,
        &format!("DEC-{}", uniq()),
        Some("2026-12-01"),
    )
    .await;
    app.create_lot(
        &token,
        &name,
        2.0,
        &format!("AUG-{}", uniq()),
        Some("2026-08-01"),
    )
    .await;
    app.create_lot(&token, &name, 2.0, &format!("NUL-{}", uniq()), None)
        .await;

    // Deduct 5kg: should take AUG (2) → DEC (2) → NULL (1), in that order.
    let resp = app.deduct(&token, &name, 5.0).await;
    assert_eq!(resp.status(), 200);
    let result: Value = resp.json().await.unwrap();
    let allocs = result["allocations"].as_array().unwrap();
    assert_eq!(allocs.len(), 3);
    assert_eq!(allocs[0]["best_before_date"], json!("2026-08-01"));
    assert_eq!(allocs[1]["best_before_date"], json!("2026-12-01"));
    assert_eq!(allocs[2]["best_before_date"], json!(null));
    assert_eq!(allocs[2]["amount_deducted"].as_f64().unwrap(), 1.0);
    assert_eq!(result["deducted_amount"].as_f64().unwrap(), 5.0);
}

#[tokio::test]
async fn fifo_same_date_breaks_tie_on_created_at_then_lot_number() {
    let app = spawn_app(false).await;
    let token = app.token().await;
    let name = format!("Saaz {}", uniq());

    // Same best-before-date; first-created should be consumed first.
    let first = app
        .create_lot(
            &token,
            &name,
            1.0,
            &format!("ZZZ-{}", uniq()),
            Some("2026-10-10"),
        )
        .await;
    let _second = app
        .create_lot(
            &token,
            &name,
            1.0,
            &format!("AAA-{}", uniq()),
            Some("2026-10-10"),
        )
        .await;

    let resp = app.deduct(&token, &name, 1.0).await;
    assert_eq!(resp.status(), 200);
    let result: Value = resp.json().await.unwrap();
    let allocs = result["allocations"].as_array().unwrap();
    assert_eq!(allocs.len(), 1);
    // Despite "ZZZ" sorting after "AAA", the earlier created_at wins the tie.
    assert_eq!(allocs[0]["lot_id"], first["id"]);
}

#[tokio::test]
async fn deduct_insufficient_stock_overdraft_off_returns_422() {
    let app = spawn_app(false).await;
    let token = app.token().await;
    let name = format!("Target {}", uniq());
    app.create_lot(
        &token,
        &name,
        1.0,
        &format!("LOT-{}", uniq()),
        Some("2026-08-01"),
    )
    .await;

    let resp = app.deduct(&token, &name, 5.0).await;
    assert_eq!(resp.status(), 422);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["code"], json!("business_rule_violation"));
    assert_eq!(body["details"]["rule"], json!("insufficient_stock"));
    assert_eq!(body["details"]["available_amount"].as_f64().unwrap(), 1.0);
    assert_eq!(body["details"]["shortage_amount"].as_f64().unwrap(), 4.0);
}

#[tokio::test]
async fn deduct_insufficient_stock_overdraft_on_warns() {
    let app = spawn_app(true).await;
    let token = app.token().await;
    let name = format!("Bramling {}", uniq());
    app.create_lot(
        &token,
        &name,
        1.0,
        &format!("LOT-{}", uniq()),
        Some("2026-08-01"),
    )
    .await;

    let resp = app.deduct(&token, &name, 5.0).await;
    assert_eq!(resp.status(), 200);
    let result: Value = resp.json().await.unwrap();
    assert_eq!(result["warning"], json!("negative_balance"));
    assert_eq!(result["deducted_amount"].as_f64().unwrap(), 5.0);
    // Overdraft drives the oldest lot negative using its original amount
    // (1.0 - 4.0 remaining = -3.0), matching the Go allocation behaviour.
    let allocs = result["allocations"].as_array().unwrap();
    let last = allocs.last().unwrap();
    assert_eq!(last["remaining_in_lot"].as_f64().unwrap(), -3.0);
}
