//! Integration tests for the recipe module against real Postgres: CRUD with
//! nested children, physics-derived calculated values, patch-recompute, and
//! BeerXML/Brewfather import.

use std::net::SocketAddr;

use base64::Engine;
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
            "email": format!("r-{}@example.com", uniq()),
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

    async fn post_json(&self, path: &str, token: &str, body: Value) -> reqwest::Response {
        self.client
            .post(format!("{}{path}", self.base))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .unwrap()
    }
}

fn sample_recipe(name: &str) -> Value {
    json!({
        "name": name,
        "type": "all_grain",
        "batch_size_liters": 20.0,
        "efficiency_pct": 75.0,
        "fermentables": [
            {"step_order": 1, "name": "Pale Malt", "amount": 5.0, "unit": "kg", "color_ebc": 7.0, "potential_ppg": 37.0}
        ],
        "hops": [
            {"step_order": 1, "name": "Cascade", "amount": 30.0, "unit": "g", "alpha_acid_pct": 6.0, "boil_time_minutes": 60.0, "use": "boil", "form": "pellet"}
        ],
        "yeasts": [
            {"name": "US-05", "amount": 11.0, "unit": "g", "attenuation_pct": 80.0}
        ],
        "mash_steps": [
            {"step_order": 1, "step_type": "infusion", "target_temp_c": 67.0, "hold_minutes": 60}
        ]
    })
}

#[tokio::test]
async fn create_computes_physics_values() {
    let app = spawn_app().await;
    let token = app.token().await;
    let resp = app
        .post_json(
            "/api/v1/recipes",
            &token,
            sample_recipe(&format!("IPA {}", uniq())),
        )
        .await;
    assert_eq!(resp.status(), 201);
    let rec: Value = resp.json().await.unwrap();

    // Physics pipeline populated the cached values.
    assert!(rec["calc_og"].as_f64().unwrap() > 1.0);
    assert!(rec["calc_fg"].as_f64().unwrap() > 1.0);
    assert!(rec["calc_abv_pct"].as_f64().unwrap() > 0.0);
    assert!(rec["calc_ibu"].as_f64().unwrap() > 0.0);
    assert!(rec["calc_color_ebc"].as_f64().unwrap() > 0.0);
    assert_eq!(rec["fermentables"].as_array().unwrap().len(), 1);
    assert_eq!(rec["hops"].as_array().unwrap().len(), 1);
    assert_eq!(rec["mash_steps"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn patch_removing_hops_recomputes_ibu_to_zero() {
    let app = spawn_app().await;
    let token = app.token().await;
    let created: Value = app
        .post_json(
            "/api/v1/recipes",
            &token,
            sample_recipe(&format!("Pale {}", uniq())),
        )
        .await
        .json()
        .await
        .unwrap();
    let id = created["id"].as_str().unwrap();
    assert!(created["calc_ibu"].as_f64().unwrap() > 0.0);

    let resp = app
        .client
        .patch(format!("{}/api/v1/recipes/{id}", app.base))
        .bearer_auth(&token)
        .json(&json!({"hops": []}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let rec: Value = resp.json().await.unwrap();
    assert_eq!(rec["calc_ibu"].as_f64().unwrap(), 0.0);
    assert_eq!(rec["hops"].as_array().unwrap().len(), 0);
    // OG unaffected by removing hops.
    assert!(rec["calc_og"].as_f64().unwrap() > 1.0);
}

#[tokio::test]
async fn list_get_and_delete() {
    let app = spawn_app().await;
    let token = app.token().await;
    let created: Value = app
        .post_json(
            "/api/v1/recipes",
            &token,
            sample_recipe(&format!("Stout {}", uniq())),
        )
        .await
        .json()
        .await
        .unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = app
        .client
        .get(format!("{}/api/v1/recipes/{id}", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = app
        .client
        .get(format!("{}/api/v1/recipes", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let page: Value = resp.json().await.unwrap();
    assert!(page["total"].as_i64().unwrap() >= 1);

    let resp = app
        .client
        .delete(format!("{}/api/v1/recipes/{id}", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let resp = app
        .client
        .get(format!("{}/api/v1/recipes/{id}", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn import_beerxml_and_brewfather() {
    let app = spawn_app().await;
    let token = app.token().await;

    // BeerXML is base64-encoded (matches the Go contract).
    let xml = include_str!("../testdata/sample.xml");
    let encoded = base64::engine::general_purpose::STANDARD.encode(xml);
    let resp = app
        .post_json(
            "/api/v1/recipes/import",
            &token,
            json!({"format": "beerxml", "data": encoded}),
        )
        .await;
    assert_eq!(resp.status(), 201, "beerxml import");
    let rec: Value = resp.json().await.unwrap();
    assert!(!rec["name"].as_str().unwrap().is_empty());
    assert!(!rec["fermentables"].as_array().unwrap().is_empty());

    // Brewfather is raw JSON.
    let bf = include_str!("../testdata/sample_brewfather.json");
    let resp = app
        .post_json(
            "/api/v1/recipes/import",
            &token,
            json!({"format": "brewfather", "data": bf}),
        )
        .await;
    assert_eq!(resp.status(), 201, "brewfather import");
    let rec: Value = resp.json().await.unwrap();
    assert!(!rec["name"].as_str().unwrap().is_empty());
}
