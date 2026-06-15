//! Integration tests for the dashboard aggregation endpoint and the public
//! OpenAPI spec / docs endpoints.

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
        resp.json::<Value>().await.unwrap()["access_token"]
            .as_str()
            .unwrap()
            .to_string()
    }
}

#[tokio::test]
async fn dashboard_aggregates_stats() {
    let app = spawn_app().await;
    let token = app.token().await;

    // Seed a recipe + a planned batch so the aggregates are non-trivial.
    let recipe: Value = app
        .client
        .post(format!("{}/api/v1/recipes", app.base))
        .bearer_auth(&token)
        .json(&json!({
            "name": format!("Recipe {}", uniq()),
            "type": "all_grain",
            "batch_size_liters": 20.0,
            "fermentables": [{"step_order": 1, "name": "Pale", "amount": 5.0, "unit": "kg", "potential_ppg": 37.0}],
            "yeasts": [{"name": "US-05", "amount": 11.0, "unit": "g"}]
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let recipe_id = recipe["id"].as_str().unwrap();

    app.client
        .post(format!("{}/api/v1/batches", app.base))
        .bearer_auth(&token)
        .json(&json!({"recipe_id": recipe_id, "batch_number": format!("B-{}", uniq()), "name": "DashBatch"}))
        .send()
        .await
        .unwrap();

    let resp = app
        .client
        .get(format!("{}/api/v1/dashboard/stats", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let stats: Value = resp.json().await.unwrap();

    assert!(stats["recipes_count"].as_i64().unwrap() >= 1);
    assert!(stats["batch_status_breakdown"]["planned"].as_i64().unwrap() >= 1);
    assert!(stats["active_batches_count"].as_i64().unwrap() >= 1);
    assert!(stats["generated_at"].is_string());
    // Home tenant: tracking / reporting add-ons are omitted.
    assert!(stats.get("containers_in_use_count").is_none());
    assert!(stats.get("last_30d_estimated_duty_pence").is_none());
}

#[tokio::test]
async fn openapi_spec_and_docs_are_public() {
    let app = spawn_app().await;

    // No auth required.
    let resp = app
        .client
        .get(format!("{}/api/v1/openapi.yaml", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert!(resp.headers()["content-type"]
        .to_str()
        .unwrap()
        .contains("yaml"));
    let body = resp.text().await.unwrap();
    assert!(body.contains("openapi:"));

    let resp = app
        .client
        .get(format!("{}/api/v1/docs", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert!(resp.text().await.unwrap().contains("swagger-ui"));
}
