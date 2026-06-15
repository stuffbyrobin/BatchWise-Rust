//! Integration tests for yeast banking (entries + propagations, `yeast_banking`
//! feature).

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
            "email": format!("y-{}@example.com", uniq()),
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
}

#[tokio::test]
async fn yeast_banking_feature_gate_blocks_home_tenant() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    // The home preset (faithful to the Go source) enables `yeast_banking` by
    // default, so disable it explicitly to exercise the gate.
    app.enable(tid, "{\"yeast_banking\":false}").await;
    let resp = app.get("/api/v1/yeast-bank", &token).await;
    assert_eq!(resp.status(), 403);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["details"]["required_feature"],
        json!("yeast_banking")
    );
}

#[tokio::test]
async fn entry_crud_and_harvest() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable(tid, "{\"yeast_banking\":true}").await;

    let harvested = (chrono::Utc::now() - chrono::Duration::days(10)).to_rfc3339();

    // Create with a harvested_at ~10 days ago.
    let resp = app
        .post(
            "/api/v1/yeast-bank",
            &token,
            json!({
                "name": format!("US-05 {}", uniq()),
                "harvested_at": harvested,
                "viability_percent": 90.0,
                "quantity_ml": 500.0
            }),
        )
        .await;
    assert_eq!(resp.status(), 201);
    assert!(resp
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("/api/v1/yeast-bank/"));
    let entry: Value = resp.json().await.unwrap();
    let id = entry["id"].as_str().unwrap().to_string();
    assert_eq!(entry["generation"], json!(1));
    assert_eq!(entry["status"], json!("active"));
    assert!(entry["days_in_storage"].as_i64().unwrap() >= 9);

    // Get.
    let got: Value = app
        .get(&format!("/api/v1/yeast-bank/{id}"), &token)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(got["id"], json!(id));
    assert!(got["days_in_storage"].as_i64().unwrap() >= 9);

    // List filtered by status.
    let list: Value = app
        .get("/api/v1/yeast-bank?status=active", &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(list["total"].as_i64().unwrap() >= 1);

    // Patch viability.
    let patched: Value = app
        .patch(
            &format!("/api/v1/yeast-bank/{id}"),
            &token,
            json!({ "viability_percent": 80.0 }),
        )
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(patched["viability_percent"], json!(80.0));

    // Harvest → generation 2, status active.
    let r = app
        .post(
            &format!("/api/v1/yeast-bank/{id}/harvest"),
            &token,
            json!({ "viability_percent": 95.0 }),
        )
        .await;
    assert_eq!(r.status(), 200);
    let harvested_entry: Value = r.json().await.unwrap();
    assert_eq!(harvested_entry["generation"], json!(2));
    assert_eq!(harvested_entry["status"], json!("active"));
    // harvested_at updated to ~now → small days_in_storage.
    assert!(harvested_entry["days_in_storage"].as_i64().unwrap() < 9);

    // Set status discarded.
    let r = app
        .patch(
            &format!("/api/v1/yeast-bank/{id}"),
            &token,
            json!({ "status": "discarded" }),
        )
        .await;
    assert_eq!(r.status(), 200);

    // Harvest from discarded → 422 discarded_terminal.
    let r = app
        .post(
            &format!("/api/v1/yeast-bank/{id}/harvest"),
            &token,
            json!({}),
        )
        .await;
    assert_eq!(r.status(), 422);
    assert_eq!(
        r.json::<Value>().await.unwrap()["details"]["rule"],
        json!("discarded_terminal")
    );

    // Patch status on a discarded entry → 422 discarded_terminal.
    let r = app
        .patch(
            &format!("/api/v1/yeast-bank/{id}"),
            &token,
            json!({ "status": "active" }),
        )
        .await;
    assert_eq!(r.status(), 422);
    assert_eq!(
        r.json::<Value>().await.unwrap()["details"]["rule"],
        json!("discarded_terminal")
    );

    // Delete → 204.
    let r = app
        .delete(&format!("/api/v1/yeast-bank/{id}"), &token)
        .await;
    assert_eq!(r.status(), 204);
}

#[tokio::test]
async fn propagations_crud() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable(tid, "{\"yeast_banking\":true}").await;

    // Create an entry.
    let entry: Value = app
        .post(
            "/api/v1/yeast-bank",
            &token,
            json!({ "name": format!("WLP001 {}", uniq()) }),
        )
        .await
        .json()
        .await
        .unwrap();
    let id = entry["id"].as_str().unwrap().to_string();

    // Create a propagation.
    let resp = app
        .post(
            &format!("/api/v1/yeast-bank/{id}/propagations"),
            &token,
            json!({ "volume_ml": 1000.0 }),
        )
        .await;
    assert_eq!(resp.status(), 201);
    assert!(resp
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with(&format!("/api/v1/yeast-bank/{id}/propagations/")));
    let prop: Value = resp.json().await.unwrap();
    let prop_id = prop["id"].as_str().unwrap().to_string();

    // Create a second so we can assert ordering (newest first).
    let prop2: Value = app
        .post(
            &format!("/api/v1/yeast-bank/{id}/propagations"),
            &token,
            json!({ "volume_ml": 2000.0 }),
        )
        .await
        .json()
        .await
        .unwrap();
    let prop2_id = prop2["id"].as_str().unwrap().to_string();

    // List: total >= 1, newest first.
    let list: Value = app
        .get(&format!("/api/v1/yeast-bank/{id}/propagations"), &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(list["total"].as_i64().unwrap() >= 1);
    assert_eq!(list["items"][0]["id"], json!(prop2_id));

    // Patch volume_ml + completed_at.
    let completed = chrono::Utc::now().to_rfc3339();
    let patched: Value = app
        .patch(
            &format!("/api/v1/yeast-bank/{id}/propagations/{prop_id}"),
            &token,
            json!({ "volume_ml": 1500.0, "completed_at": completed }),
        )
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(patched["volume_ml"], json!(1500.0));
    assert!(patched["completed_at"].is_string());

    // Delete → 204.
    let r = app
        .delete(
            &format!("/api/v1/yeast-bank/{id}/propagations/{prop_id}"),
            &token,
        )
        .await;
    assert_eq!(r.status(), 204);

    // Deleting again → 404.
    let r = app
        .delete(
            &format!("/api/v1/yeast-bank/{id}/propagations/{prop_id}"),
            &token,
        )
        .await;
    assert_eq!(r.status(), 404);

    // Creating a propagation under a non-existent bank id → 404.
    let bogus = Uuid::new_v4();
    let r = app
        .post(
            &format!("/api/v1/yeast-bank/{bogus}/propagations"),
            &token,
            json!({ "volume_ml": 100.0 }),
        )
        .await;
    assert_eq!(r.status(), 404);
}
