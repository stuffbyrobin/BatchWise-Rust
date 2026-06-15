//! Integration tests for the water-chemistry module: water-profile CRUD, the
//! `/calculate` endpoint (backed by `pkg::water`), and water-adjustment CRUD.

use std::net::SocketAddr;

use batchwise::platform::config::Config;
use batchwise::platform::database;
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
    let state = AppState::new(pool, test_config(url));
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
    async fn token(&self) -> String {
        let body = json!({
            "email": format!("w-{}@example.com", uniq()),
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

fn distilled() -> Value {
    json!({"calcium_ppm": 0.0, "magnesium_ppm": 0.0, "sodium_ppm": 0.0, "sulfate_ppm": 0.0, "chloride_ppm": 0.0, "bicarbonate_ppm": 0.0})
}

#[tokio::test]
async fn water_profile_crud() {
    let app = spawn_app().await;
    let token = app.token().await;

    let resp = app
        .post(
            "/api/v1/water-profiles",
            &token,
            json!({
                "name": format!("Burton {}", uniq()),
                "calcium_ppm": 275.0, "magnesium_ppm": 40.0, "sodium_ppm": 25.0,
                "sulfate_ppm": 610.0, "chloride_ppm": 35.0, "bicarbonate_ppm": 270.0
            }),
        )
        .await;
    assert_eq!(resp.status(), 201);
    let p: Value = resp.json().await.unwrap();
    let id = p["id"].as_str().unwrap();

    assert_eq!(
        app.get(&format!("/api/v1/water-profiles/{id}"), &token)
            .await
            .status(),
        200
    );
    let page: Value = app
        .get("/api/v1/water-profiles", &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(page["total"].as_i64().unwrap() >= 1);
}

#[tokio::test]
async fn calculate_adds_minerals() {
    let app = spawn_app().await;
    let token = app.token().await;

    // Distilled water + 5 g gypsum (CaSO4) in 20 L raises calcium and sulfate.
    let resp = app
        .post(
            "/api/v1/water-adjustments/calculate",
            &token,
            json!({
                "source_profile": distilled(),
                "volume_liters": 20.0,
                "mineral_additions": [{"type": "CaSO4", "amount": 5.0}]
            }),
        )
        .await;
    assert_eq!(resp.status(), 200, "calculate");
    let r: Value = resp.json().await.unwrap();
    assert!(r["calcium_ppm"].as_f64().unwrap() > 0.0);
    assert!(r["sulfate_ppm"].as_f64().unwrap() > 0.0);
    // Gypsum adds no chloride.
    assert_eq!(r["chloride_ppm"].as_f64().unwrap(), 0.0);
}

#[tokio::test]
async fn water_adjustment_crud() {
    let app = spawn_app().await;
    let token = app.token().await;

    let profile: Value = app
        .post(
            "/api/v1/water-profiles",
            &token,
            json!({
                "name": format!("Soft {}", uniq()),
                "calcium_ppm": 10.0, "magnesium_ppm": 2.0, "sodium_ppm": 5.0,
                "sulfate_ppm": 8.0, "chloride_ppm": 6.0, "bicarbonate_ppm": 15.0
            }),
        )
        .await
        .json()
        .await
        .unwrap();
    let source_id = profile["id"].as_str().unwrap();

    let resp = app
        .post(
            "/api/v1/water-adjustments",
            &token,
            json!({
                "name": format!("Adj {}", uniq()),
                "source_profile_id": source_id,
                "volume_liters": 25.0,
                "mineral_additions": [{"type": "CaCl2", "amount": 3.0}]
            }),
        )
        .await;
    assert_eq!(resp.status(), 201, "create adjustment");
    let adj: Value = resp.json().await.unwrap();
    let id = adj["id"].as_str().unwrap();

    assert_eq!(
        app.get(&format!("/api/v1/water-adjustments/{id}"), &token)
            .await
            .status(),
        200
    );
    let page: Value = app
        .get("/api/v1/water-adjustments", &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(page["total"].as_i64().unwrap() >= 1);
}
