//! Integration tests for packaging (runs + distribution movements, `packaging`
//! feature) and traceability (forward/backward/recall, `traceability` feature).

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
    async fn register(&self) -> (String, Uuid) {
        let body = json!({
            "email": format!("p-{}@example.com", uniq()),
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
        (
            v["access_token"].as_str().unwrap().to_string(),
            Uuid::parse_str(v["tenant_id"].as_str().unwrap()).unwrap(),
        )
    }

    async fn enable(&self, tenant_id: Uuid, flags: &str) {
        let pool = sqlx::PgPool::connect(&self.db_url).await.unwrap();
        sqlx::query("UPDATE tenants SET feature_flags = feature_flags || $2::jsonb WHERE id=$1")
            .bind(tenant_id)
            .bind(flags)
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

    /// Creates a recipe (single fermentable), batch, stocks that ingredient, and
    /// brews the batch so a `batch_ingredients` row links the lot. Returns
    /// (batch_id, ingredient lot_number, ingredient name).
    async fn brewed_batch(&self, token: &str) -> (String, String, String) {
        let name = format!("Malt {}", uniq());
        let lot_number = format!("TRACE-{}", uniq());
        let recipe: Value = self
            .post("/api/v1/recipes", token, json!({
                "name": format!("Trace Ale {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0,
                "fermentables": [{"step_order": 1, "name": name, "amount": 5.0, "unit": "kg", "potential_ppg": 37.0}]
            }))
            .await.json().await.unwrap();
        let batch: Value = self
            .post("/api/v1/batches", token, json!({"recipe_id": recipe["id"], "batch_number": format!("B-{}", uniq()), "name": "Trace Batch"}))
            .await.json().await.unwrap();
        let bid = batch["batch"]["id"].as_str().unwrap().to_string();
        // Stock the fermentable so brewing can deduct it (creating batch_ingredients).
        let resp = self.post("/api/v1/inventory", token, json!({"type": "fermentable", "name": name, "amount": 100.0, "unit": "kg", "lot_number": lot_number})).await;
        assert_eq!(resp.status(), 201);
        assert_eq!(
            self.post(
                &format!("/api/v1/batches/{bid}/transition"),
                token,
                json!({"to_status": "brewing"})
            )
            .await
            .status(),
            200
        );
        (bid, lot_number, name)
    }

    async fn packaging_run(&self, token: &str, batch_id: &str) -> String {
        let resp = self
            .post(
                "/api/v1/packaging-runs",
                token,
                json!({
                    "batch_id": batch_id, "format": "keg", "unit_volume_ml": 50000, "quantity": 100,
                    "lot_number": format!("PKG-{}", uniq()), "packaged_at": "2026-07-01"
                }),
            )
            .await;
        assert_eq!(resp.status(), 201, "create packaging run");
        resp.json::<Value>().await.unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string()
    }
}

#[tokio::test]
async fn packaging_feature_gate_blocks_home_tenant() {
    let app = spawn_app().await;
    let (token, _tid) = app.register().await;
    let resp = app.get("/api/v1/packaging-runs", &token).await;
    assert_eq!(resp.status(), 403);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["details"]["required_feature"],
        json!("packaging")
    );
    let resp = app
        .get("/api/v1/traceability/recall?lot_number=x", &token)
        .await;
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn packaging_run_and_movement_stock() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable(tid, "{\"packaging\":true}").await;
    let (bid, _lot, _name) = app.brewed_batch(&token).await;
    let run_id = app.packaging_run(&token, &bid).await;

    // A sale movement of 10 leaves 90 in stock.
    let resp = app
        .post("/api/v1/distribution-movements", &token, json!({"packaging_run_id": run_id, "movement_type": "sample", "quantity": 10, "to_location": "Taproom"}))
        .await;
    assert_eq!(resp.status(), 201, "create movement");
    let run: Value = app
        .get(&format!("/api/v1/packaging-runs/{run_id}"), &token)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(run["stock_remaining"].as_i64().unwrap(), 90);

    // Moving more than remaining is rejected.
    let resp = app
        .post("/api/v1/distribution-movements", &token, json!({"packaging_run_id": run_id, "movement_type": "sample", "quantity": 1000, "to_location": "Taproom"}))
        .await;
    assert_eq!(resp.status(), 422);
}

#[tokio::test]
async fn traceability_forward_backward_and_recall() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable(tid, "{\"packaging\":true,\"traceability\":true}")
        .await;
    let (bid, lot, _name) = app.brewed_batch(&token).await;
    let run_id = app.packaging_run(&token, &bid).await;

    // Backward: packaging run → batch → ingredient lots.
    let back: Value = app
        .get(
            &format!("/api/v1/traceability/packaging-runs/{run_id}"),
            &token,
        )
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(back["batch"]["batch_id"], json!(bid));
    assert!(!back["ingredient_lots"].as_array().unwrap().is_empty());

    // Forward: ingredient lot_number → batches → packaging runs.
    let fwd = app
        .get(
            &format!("/api/v1/traceability/ingredient-lots/{lot}"),
            &token,
        )
        .await;
    assert_eq!(fwd.status(), 200);
    let fwd: Value = fwd.json().await.unwrap();
    let batches = fwd["batches"].as_array().unwrap();
    assert!(batches.iter().any(|b| b["batch"]["batch_id"] == json!(bid)));

    // Recall by lot_number → affected batch + packaging-run counts.
    let recall: Value = app
        .get(
            &format!("/api/v1/traceability/recall?lot_number={lot}"),
            &token,
        )
        .await
        .json()
        .await
        .unwrap();
    assert!(recall["affected_batches"].as_i64().unwrap() >= 1);
    assert!(recall["affected_packaging_runs"].as_i64().unwrap() >= 1);
}
