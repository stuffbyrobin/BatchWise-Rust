//! Integration tests for batches, calendar, and yeast kinetics against real
//! Postgres: batch creation with calendar-event generation, the status FSM
//! (valid + invalid transitions), deferred inventory deduction on brewing, and
//! calendar / yeast-kinetics CRUD.

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
    seed::run(&pool).await.expect("seed");
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
            "email": format!("b-{}@example.com", uniq()),
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

    /// Creates a recipe whose ingredient names embed `base`; returns its id.
    async fn create_recipe(&self, token: &str, base: &str) -> String {
        let body = json!({
            "name": format!("Recipe {base}"),
            "type": "all_grain",
            "batch_size_liters": 20.0,
            "efficiency_pct": 75.0,
            "fermentables": [{"step_order": 1, "name": format!("Malt {base}"), "amount": 5.0, "unit": "kg", "color_ebc": 7.0, "potential_ppg": 37.0}],
            "hops": [{"step_order": 1, "name": format!("Hop {base}"), "amount": 30.0, "unit": "g", "alpha_acid_pct": 6.0, "boil_time_minutes": 60.0, "use": "boil", "form": "pellet"}],
            "yeasts": [{"name": format!("Yeast {base}"), "amount": 11.0, "unit": "g", "attenuation_pct": 80.0}],
            "mash_steps": [{"step_order": 1, "step_type": "infusion", "target_temp_c": 67.0, "hold_minutes": 60}]
        });
        let resp = self.post("/api/v1/recipes", token, body).await;
        assert_eq!(resp.status(), 201);
        resp.json::<Value>().await.unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    async fn create_lot(&self, token: &str, typ: &str, name: &str, unit: &str, amount: f64) {
        let body = json!({
            "type": typ, "name": name, "amount": amount, "unit": unit,
            "lot_number": format!("LOT-{}", uniq())
        });
        let resp = self.post("/api/v1/inventory", token, body).await;
        assert_eq!(resp.status(), 201, "create lot {name}");
    }

    async fn transition(&self, token: &str, batch_id: &str, to: &str) -> reqwest::Response {
        self.post(
            &format!("/api/v1/batches/{batch_id}/transition"),
            token,
            json!({"to_status": to}),
        )
        .await
    }
}

#[tokio::test]
async fn create_batch_generates_calendar_events() {
    let app = spawn_app().await;
    let token = app.token().await;
    let recipe_id = app.create_recipe(&token, &uniq()).await;

    let resp = app
        .post("/api/v1/batches", &token, json!({"recipe_id": recipe_id, "batch_number": format!("B-{}", uniq()), "name": "Test Batch", "brew_date": "2026-07-01"}))
        .await;
    assert_eq!(resp.status(), 201);
    assert!(resp.headers().contains_key("location"));
    let result: Value = resp.json().await.unwrap();
    assert_eq!(result["batch"]["status"], json!("planned"));
    assert_eq!(result["batch"]["duty_status"], json!("suspended"));
    // Snapshot captured the recipe.
    assert!(
        result["batch"]["batch_recipe_snapshot"]["fermentables"]
            .as_array()
            .unwrap()
            .len()
            == 1
    );
    // No yeast_id on the recipe yeast → default 2-event timeline.
    let events = result["generated_calendar_events"].as_array().unwrap();
    assert_eq!(events.len(), 2);
    let types: Vec<&str> = events
        .iter()
        .map(|e| e["event_type"].as_str().unwrap())
        .collect();
    assert!(types.contains(&"brew_day") && types.contains(&"package"));

    // Those events are queryable via the calendar endpoint.
    let bid = result["batch"]["id"].as_str().unwrap();
    let page: Value = app
        .get(&format!("/api/v1/calendar-events?batch_id={bid}"), &token)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(page["total"].as_i64().unwrap(), 2);
}

#[tokio::test]
async fn fsm_rejects_invalid_transitions() {
    let app = spawn_app().await;
    let token = app.token().await;
    let recipe_id = app.create_recipe(&token, &uniq()).await;
    let batch: Value = app
        .post(
            "/api/v1/batches",
            &token,
            json!({"recipe_id": recipe_id, "batch_number": format!("B-{}", uniq()), "name": "FSM"}),
        )
        .await
        .json()
        .await
        .unwrap();
    let id = batch["batch"]["id"].as_str().unwrap();

    // planned -> completed is not allowed (skips states).
    let resp = app.transition(&token, id, "completed").await;
    assert_eq!(resp.status(), 422);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["details"]["rule"], json!("invalid_status_transition"));

    // planned -> fermenting is not allowed either.
    assert_eq!(app.transition(&token, id, "fermenting").await.status(), 422);

    // planned -> cancelled IS allowed.
    assert_eq!(app.transition(&token, id, "cancelled").await.status(), 200);
}

#[tokio::test]
async fn brewing_transition_deducts_inventory() {
    let app = spawn_app().await;
    let token = app.token().await;
    let base = uniq();
    let recipe_id = app.create_recipe(&token, &base).await;

    // Stock the three snapshot ingredients (matched by type+name+unit).
    app.create_lot(&token, "fermentable", &format!("Malt {base}"), "kg", 10.0)
        .await;
    app.create_lot(&token, "hop", &format!("Hop {base}"), "g", 100.0)
        .await;
    app.create_lot(&token, "yeast", &format!("Yeast {base}"), "g", 50.0)
        .await;

    let batch: Value = app
        .post("/api/v1/batches", &token, json!({"recipe_id": recipe_id, "batch_number": format!("B-{}", uniq()), "name": "Brew"}))
        .await
        .json()
        .await
        .unwrap();
    let id = batch["batch"]["id"].as_str().unwrap();

    // planned -> brewing deducts inventory and succeeds.
    let resp = app.transition(&token, id, "brewing").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["status"],
        json!("brewing")
    );

    // The fermentable lot was reduced 10 -> 5.
    let page: Value = app
        .get(&format!("/api/v1/inventory?name=Malt {base}"), &token)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(page["items"][0]["amount"].as_f64().unwrap(), 5.0);
}

#[tokio::test]
async fn brewing_without_stock_fails() {
    let app = spawn_app().await;
    let token = app.token().await;
    let recipe_id = app.create_recipe(&token, &uniq()).await;
    let batch: Value = app
        .post("/api/v1/batches", &token, json!({"recipe_id": recipe_id, "batch_number": format!("B-{}", uniq()), "name": "NoStock"}))
        .await
        .json()
        .await
        .unwrap();
    let id = batch["batch"]["id"].as_str().unwrap();

    // No inventory exists for the snapshot ingredients → 422 insufficient_stock.
    let resp = app.transition(&token, id, "brewing").await;
    assert_eq!(resp.status(), 422);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["details"]["rule"],
        json!("insufficient_stock")
    );
}

#[tokio::test]
async fn calendar_event_crud() {
    let app = spawn_app().await;
    let token = app.token().await;

    let resp = app
        .post("/api/v1/calendar-events", &token, json!({"event_type": "custom", "title": "Clean kegs", "start_time": "2026-07-01T09:00:00Z"}))
        .await;
    assert_eq!(resp.status(), 201);
    let ev: Value = resp.json().await.unwrap();
    let id = ev["id"].as_str().unwrap();
    assert_eq!(ev["status"], json!("pending"));

    let resp = app
        .client
        .patch(format!("{}/api/v1/calendar-events/{id}", app.base))
        .bearer_auth(&token)
        .json(&json!({"status": "completed"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["status"],
        json!("completed")
    );

    assert_eq!(
        app.get(&format!("/api/v1/calendar-events/{id}"), &token)
            .await
            .status(),
        200
    );
}

#[tokio::test]
async fn yeast_kinetics_crud() {
    let app = spawn_app().await;
    let token = app.token().await;

    // Use a seeded system yeast id (FK target for kinetics.yeast_id).
    let yeasts: Value = app
        .get("/api/v1/library/yeasts", &token)
        .await
        .json()
        .await
        .unwrap();
    let yeast_id = yeasts["items"][0]["id"]
        .as_str()
        .expect("a seeded yeast")
        .to_string();

    let resp = app
        .post("/api/v1/yeast-kinetics", &token, json!({"yeast_id": yeast_id, "fermentation_temp_c": 18.0, "primary_fermentation_days": 7, "conditioning_days": 14, "lag_phase_hours": 12, "attenuation_pct": 78.0}))
        .await;
    assert_eq!(resp.status(), 201, "create kinetics");
    let k: Value = resp.json().await.unwrap();
    assert_eq!(k["primary_fermentation_days"], json!(7));
    let id = k["id"].as_str().unwrap();

    assert_eq!(
        app.get(&format!("/api/v1/yeast-kinetics/{id}"), &token)
            .await
            .status(),
        200
    );
    let page: Value = app
        .get("/api/v1/yeast-kinetics", &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(page["total"].as_i64().unwrap() >= 1);
}
