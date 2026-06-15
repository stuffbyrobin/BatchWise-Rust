//! Integration tests for the fermentation module
//! (`/batches/{id}/fermentation`, `fermentation`-feature-gated): readings logged
//! against a batch.

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
            "email": format!("f-{}@example.com", uniq()),
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

    async fn set_flags(&self, tenant_id: Uuid, flags: &str) {
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
    async fn patch(&self, path: &str, token: &str, body: Value) -> reqwest::Response {
        self.client
            .patch(format!("{}{path}", self.base))
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
    async fn delete(&self, path: &str, token: &str) -> reqwest::Response {
        self.client
            .delete(format!("{}{path}", self.base))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
    }

    async fn make_batch(&self, token: &str) -> String {
        let recipe: Value = self
            .post("/api/v1/recipes", token, json!({
                "name": format!("Ferm Ale {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0,
                "fermentables": [{"step_order": 1, "name": "Pale", "amount": 5.0, "unit": "kg", "potential_ppg": 37.0}]
            }))
            .await.json().await.unwrap();
        let batch: Value = self
            .post("/api/v1/batches", token, json!({"recipe_id": recipe["id"], "batch_number": format!("B-{}", uniq()), "name": "Ferm Batch"}))
            .await.json().await.unwrap();
        batch["batch"]["id"].as_str().unwrap().to_string()
    }
}

#[tokio::test]
async fn fermentation_feature_gate_blocks_when_disabled() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    let batch_id = app.make_batch(&token).await;
    // `fermentation` is on by default for the home tier; disable it to assert the gate.
    app.set_flags(tid, "{\"fermentation\":false}").await;
    let resp = app
        .get(&format!("/api/v1/batches/{batch_id}/fermentation"), &token)
        .await;
    assert_eq!(resp.status(), 403);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["details"]["required_feature"],
        json!("fermentation")
    );
}

#[tokio::test]
async fn reading_crud_lifecycle() {
    let app = spawn_app().await;
    let (token, _tid) = app.register().await;
    let batch_id = app.make_batch(&token).await;

    // Create a reading (stage defaults to "primary").
    let resp = app
        .post(
            &format!("/api/v1/batches/{batch_id}/fermentation"),
            &token,
            json!({"gravity": 1.048, "temp_c": 19.5, "ph": 5.2}),
        )
        .await;
    assert_eq!(resp.status(), 201, "create reading");
    let rd: Value = resp.json().await.unwrap();
    let id = rd["id"].as_str().unwrap().to_string();
    assert_eq!(rd["stage"], json!("primary"));
    assert_eq!(rd["gravity"].as_f64().unwrap(), 1.048);

    // A second reading at a different stage.
    assert_eq!(
        app.post(
            &format!("/api/v1/batches/{batch_id}/fermentation"),
            &token,
            json!({"stage": "secondary", "gravity": 1.012})
        )
        .await
        .status(),
        201
    );

    // List (newest first) and filter by stage.
    let page: Value = app
        .get(&format!("/api/v1/batches/{batch_id}/fermentation"), &token)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(page["total"].as_i64().unwrap(), 2);
    let filtered: Value = app
        .get(
            &format!("/api/v1/batches/{batch_id}/fermentation?stage=secondary"),
            &token,
        )
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(filtered["total"].as_i64().unwrap(), 1);

    // Patch the first reading.
    let resp = app
        .patch(
            &format!("/api/v1/batches/{batch_id}/fermentation/{id}"),
            &token,
            json!({"gravity": 1.010, "notes": "FG reached"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let patched: Value = resp.json().await.unwrap();
    assert_eq!(patched["gravity"].as_f64().unwrap(), 1.010);
    assert_eq!(patched["notes"], json!("FG reached"));

    // Invalid pH is rejected (field validation → 400).
    assert_eq!(
        app.post(
            &format!("/api/v1/batches/{batch_id}/fermentation"),
            &token,
            json!({"ph": 20.0})
        )
        .await
        .status(),
        400
    );

    // Delete, then a second delete 404s.
    assert_eq!(
        app.delete(
            &format!("/api/v1/batches/{batch_id}/fermentation/{id}"),
            &token
        )
        .await
        .status(),
        204
    );
    assert_eq!(
        app.delete(
            &format!("/api/v1/batches/{batch_id}/fermentation/{id}"),
            &token
        )
        .await
        .status(),
        404
    );

    // Readings against a non-existent batch → 404 batch.
    assert_eq!(
        app.get(
            &format!("/api/v1/batches/{}/fermentation", Uuid::new_v4()),
            &token
        )
        .await
        .status(),
        404
    );
}
