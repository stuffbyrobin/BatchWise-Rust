//! Integration tests for the duty & excise records module (`/duty-returns`,
//! `duty`-feature-gated): the tier gate, and compiling a return that aggregates
//! the duty events crystallised by a sales fulfilment.

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
            "email": format!("d-{}@example.com", uniq()),
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

    async fn enable(&self, tenant_id: Uuid, flags_json: &str) {
        let pool = sqlx::PgPool::connect(&self.db_url).await.unwrap();
        sqlx::query("UPDATE tenants SET feature_flags = feature_flags || $2::jsonb WHERE id=$1")
            .bind(tenant_id)
            .bind(flags_json)
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

    async fn post_empty(&self, path: &str, token: &str) -> reqwest::Response {
        self.client
            .post(format!("{}{path}", self.base))
            .bearer_auth(token)
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

    /// Drives a sale to fulfilment so a duty_event is crystallised; returns nothing.
    async fn crystallise_duty_event(&self, token: &str) {
        let recipe: Value = self
            .post("/api/v1/recipes", token, json!({"name": format!("R {}", uniq()), "type": "all_grain", "batch_size_liters": 100.0, "yeasts": [{"name": "US-05", "amount": 11.0, "unit": "g"}]}))
            .await
            .json()
            .await
            .unwrap();
        let batch: Value = self
            .post("/api/v1/batches", token, json!({"recipe_id": recipe["id"], "batch_number": format!("B-{}", uniq()), "name": "Duty Batch"}))
            .await
            .json()
            .await
            .unwrap();
        let bid = batch["batch"]["id"].as_str().unwrap();
        let resp = self
            .client
            .patch(format!("{}/api/v1/batches/{bid}", self.base))
            .bearer_auth(token)
            .json(&json!({"actual_og": 1.050, "actual_fg": 1.010}))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        let customer: Value = self
            .post(
                "/api/v1/customers",
                token,
                json!({"name": format!("C {}", uniq()), "country": "GB"}),
            )
            .await
            .json()
            .await
            .unwrap();
        let order: Value = self
            .post(
                "/api/v1/orders",
                token,
                json!({"customer_id": customer["id"]}),
            )
            .await
            .json()
            .await
            .unwrap();
        let oid = order["id"].as_str().unwrap();
        let resp = self.post(&format!("/api/v1/orders/{oid}/items"), token, json!({"batch_id": bid, "product_name": "Keg", "volume_liters": 50.0, "unit_price_pence": 10000, "quantity": 2})).await;
        assert_eq!(resp.status(), 201);
        assert_eq!(
            self.post_empty(&format!("/api/v1/orders/{oid}/confirm"), token)
                .await
                .status(),
            200
        );
        assert_eq!(
            self.post(&format!("/api/v1/orders/{oid}/fulfill"), token, json!({}))
                .await
                .status(),
            200
        );
    }
}

#[tokio::test]
async fn duty_feature_gate_blocks_home_tenant() {
    let app = spawn_app().await;
    let (token, _tid) = app.register().await;
    let resp = app.get("/api/v1/duty-returns", &token).await;
    assert_eq!(resp.status(), 403);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["details"]["required_feature"],
        json!("duty")
    );
}

#[tokio::test]
async fn compile_return_aggregates_duty_events() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable(tid, "{\"sales\":true,\"duty\":true}").await;

    app.crystallise_duty_event(&token).await;

    // Compile a return over a wide window covering today's crystallisation.
    let resp = app
        .post(
            "/api/v1/duty-returns/compile",
            &token,
            json!({"period_start": "2020-01-01", "period_end": "2035-12-31"}),
        )
        .await;
    assert_eq!(resp.status(), 200, "compile");
    let ret: Value = resp.json().await.unwrap();
    assert!(
        ret["event_count"].as_i64().unwrap() >= 1,
        "expected aggregated duty events"
    );
    assert!(ret["gross_duty_pence"].as_i64().unwrap() > 0);
    assert!(ret["net_duty_pence"].as_i64().is_some());
    let id = ret["id"].as_str().unwrap();

    // It appears in the list.
    let page: Value = app
        .get("/api/v1/duty-returns", &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(page["total"].as_i64().unwrap() >= 1);

    // Submit it.
    let resp = app
        .client
        .patch(format!("{}/api/v1/duty-returns/{id}", app.base))
        .bearer_auth(&token)
        .json(&json!({"status": "submitted"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["status"],
        json!("submitted")
    );
}
