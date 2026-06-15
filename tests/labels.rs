//! Integration tests for the label-compliance module (`/label-records`,
//! `labels`-feature-gated): the tier gate and label-record generation
//! (auto-populated from batch snapshot + tenant + allergens + nutrition).

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
            "email": format!("l-{}@example.com", uniq()),
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

    async fn enable_labels(&self, tenant_id: Uuid) {
        let pool = sqlx::PgPool::connect(&self.db_url).await.unwrap();
        sqlx::query("UPDATE tenants SET feature_flags = feature_flags || '{\"labels\":true}'::jsonb WHERE id=$1")
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

    async fn make_batch(&self, token: &str) -> String {
        let recipe: Value = self
            .post("/api/v1/recipes", token, json!({"name": format!("Label Ale {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0, "yeasts": [{"name": "US-05", "amount": 11.0, "unit": "g", "attenuation_pct": 80.0}], "fermentables": [{"step_order": 1, "name": "Pale", "amount": 5.0, "unit": "kg", "potential_ppg": 37.0}]}))
            .await
            .json()
            .await
            .unwrap();
        let batch: Value = self
            .post("/api/v1/batches", token, json!({"recipe_id": recipe["id"], "batch_number": format!("B-{}", uniq()), "name": "Label Batch"}))
            .await
            .json()
            .await
            .unwrap();
        batch["batch"]["id"].as_str().unwrap().to_string()
    }
}

#[tokio::test]
async fn labels_feature_gate_blocks_home_tenant() {
    let app = spawn_app().await;
    let (token, _tid) = app.register().await;
    let resp = app.get("/api/v1/label-records", &token).await;
    assert_eq!(resp.status(), 403);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["details"]["required_feature"],
        json!("labels")
    );
}

#[tokio::test]
async fn label_record_autopopulates_from_batch() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable_labels(tid).await;
    let batch_id = app.make_batch(&token).await;

    let resp = app
        .post(
            "/api/v1/label-records",
            &token,
            json!({"batch_id": batch_id, "net_volume_ml": 500}),
        )
        .await;
    assert_eq!(resp.status(), 201, "create label record");
    let rec: Value = resp.json().await.unwrap();
    let id = rec["id"].as_str().unwrap();
    // Auto-populated from the batch snapshot + tenant.
    assert!(rec["product_name"].as_str().is_some_and(|s| !s.is_empty()));
    assert!(rec["responsible_party"]
        .as_str()
        .is_some_and(|s| !s.is_empty()));
    assert!(rec["abv_percent"].as_f64().unwrap() > 0.0);

    assert_eq!(
        app.get(&format!("/api/v1/label-records/{id}"), &token)
            .await
            .status(),
        200
    );
    let page: Value = app
        .get("/api/v1/label-records", &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(page["total"].as_i64().unwrap() >= 1);

    let resp = app
        .client
        .delete(format!("{}/api/v1/label-records/{id}", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);
}
