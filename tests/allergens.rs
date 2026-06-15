//! Integration tests for the allergen declaration endpoint
//! (`GET /recipes/{id}/allergens`, `allergens`-feature-gated). Also confirms the
//! route merges into the recipe nest without a router conflict.

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
            "email": format!("a-{}@example.com", uniq()),
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

    async fn enable_allergens(&self, tenant_id: Uuid) {
        let pool = sqlx::PgPool::connect(&self.db_url).await.unwrap();
        sqlx::query("UPDATE tenants SET feature_flags = feature_flags || '{\"allergens\":true}'::jsonb WHERE id=$1")
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
async fn allergens_feature_gate_blocks_home_tenant() {
    let app = spawn_app().await;
    let (token, _tid) = app.register().await;
    let resp = app
        .get(
            &format!("/api/v1/recipes/{}/allergens", Uuid::new_v4()),
            &token,
        )
        .await;
    assert_eq!(resp.status(), 403);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["details"]["required_feature"],
        json!("allergens")
    );
}

#[tokio::test]
async fn computes_allergens_from_matching_inventory_lots() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable_allergens(tid).await;

    let malt = format!("Pale Malt {}", uniq());
    let hop = format!("Cascade {}", uniq());

    // An inventory lot carrying allergens, matched by name to a recipe fermentable.
    let resp = app
        .post(
            "/api/v1/inventory",
            &token,
            json!({
                "type": "fermentable", "name": malt, "amount": 10.0, "unit": "kg",
                "lot_number": format!("LOT-{}", uniq()), "allergens": ["gluten", "barley"]
            }),
        )
        .await;
    assert_eq!(resp.status(), 201);

    // Recipe: one matching fermentable, one hop with no allergen lot (→ unmatched).
    let recipe: Value = app
        .post("/api/v1/recipes", &token, json!({
            "name": format!("Allergen Ale {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0,
            "fermentables": [{"step_order": 1, "name": malt, "amount": 5.0, "unit": "kg"}],
            "hops": [{"step_order": 1, "name": hop, "amount": 30.0, "unit": "g", "alpha_acid_pct": 6.0, "boil_time_minutes": 60.0}]
        }))
        .await
        .json()
        .await
        .unwrap();
    let rid = recipe["id"].as_str().unwrap();

    let resp = app
        .get(&format!("/api/v1/recipes/{rid}/allergens"), &token)
        .await;
    assert_eq!(resp.status(), 200);
    let result: Value = resp.json().await.unwrap();
    let allergens: Vec<&str> = result["allergens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(allergens.contains(&"gluten") && allergens.contains(&"barley"));
    assert_eq!(result["ingredient_names"], json!([malt]));
    assert_eq!(result["unmatched"], json!([hop]));
}
