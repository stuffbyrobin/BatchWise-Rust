//! Integration tests for the compliance-audit log (`/compliance-audit`,
//! read-only, NOT feature-gated): cross-cutting audit events are written
//! fire-and-forget by other modules and read back here.

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

    async fn make_batch(&self, token: &str) -> String {
        let recipe: Value = self
            .post("/api/v1/recipes", token, json!({
                "name": format!("Audit Ale {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0,
                "yeasts": [{"name": "US-05", "amount": 11.0, "unit": "g", "attenuation_pct": 80.0}],
                "fermentables": [{"step_order": 1, "name": "Pale", "amount": 5.0, "unit": "kg", "potential_ppg": 37.0}]
            }))
            .await.json().await.unwrap();
        let batch: Value = self
            .post("/api/v1/batches", token, json!({"recipe_id": recipe["id"], "batch_number": format!("B-{}", uniq()), "name": "Audit Batch"}))
            .await.json().await.unwrap();
        batch["batch"]["id"].as_str().unwrap().to_string()
    }
}

#[tokio::test]
async fn audit_endpoint_requires_auth_but_not_a_feature() {
    let app = spawn_app().await;
    // No token → 401.
    let resp = app
        .client
        .get(format!("{}/api/v1/compliance-audit", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // A plain home-tier tenant with no feature flags can still read the log.
    let (token, _tid) = app.register().await;
    let resp = app.get("/api/v1/compliance-audit", &token).await;
    assert_eq!(resp.status(), 200);
    let page: Value = resp.json().await.unwrap();
    assert_eq!(page["total"].as_i64().unwrap(), 0);

    // Invalid RFC3339 `from` is a validation error.
    let resp = app
        .get("/api/v1/compliance-audit?from=not-a-date", &token)
        .await;
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn audit_records_label_and_allergen_events() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable(tid, "{\"labels\":true,\"allergens\":true}")
        .await;
    let batch_id = app.make_batch(&token).await;

    // Creating a label record fires two audit events: the allergen computation
    // (during auto-population) and the label_record.created event.
    let rec: Value = app
        .post(
            "/api/v1/label-records",
            &token,
            json!({"batch_id": batch_id, "net_volume_ml": 500}),
        )
        .await
        .json()
        .await
        .unwrap();
    let label_id = rec["id"].as_str().unwrap();

    let page: Value = app
        .get("/api/v1/compliance-audit", &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(page["total"].as_i64().unwrap() >= 2);
    let items = page["items"].as_array().unwrap();
    let label_event = items
        .iter()
        .find(|e| e["event_type"] == json!("label_record.created"))
        .expect("label event");
    assert_eq!(label_event["entity_type"], json!("label_record"));
    assert_eq!(label_event["entity_id"], json!(label_id));
    // The acting user is recorded.
    assert!(label_event["actor_user_id"].as_str().is_some());
    assert_eq!(label_event["event_data"]["net_volume_ml"], json!(500));
    assert!(items
        .iter()
        .any(|e| e["event_type"] == json!("allergen_result.computed")));

    // Filter by event_type narrows the result set.
    let filtered: Value = app
        .get(
            "/api/v1/compliance-audit?event_type=label_record.created",
            &token,
        )
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(filtered["total"].as_i64().unwrap(), 1);
    assert_eq!(
        filtered["items"][0]["event_type"],
        json!("label_record.created")
    );

    // Filter by entity_type=recipe finds the allergen event.
    let by_entity: Value = app
        .get("/api/v1/compliance-audit?entity_type=recipe", &token)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(by_entity["total"].as_i64().unwrap(), 1);

    // Fetch a single event by id.
    let event_id = label_event["id"].as_str().unwrap();
    let one = app
        .get(&format!("/api/v1/compliance-audit/{event_id}"), &token)
        .await;
    assert_eq!(one.status(), 200);
    assert_eq!(
        one.json::<Value>().await.unwrap()["event_type"],
        json!("label_record.created")
    );

    // Unknown id → 404.
    assert_eq!(
        app.get(
            &format!("/api/v1/compliance-audit/{}", Uuid::new_v4()),
            &token
        )
        .await
        .status(),
        404
    );
}

#[tokio::test]
async fn audit_is_tenant_scoped() {
    let app = spawn_app().await;
    let (token_a, tid_a) = app.register().await;
    app.enable(tid_a, "{\"labels\":true,\"allergens\":true}")
        .await;
    let batch_id = app.make_batch(&token_a).await;
    app.post(
        "/api/v1/label-records",
        &token_a,
        json!({"batch_id": batch_id, "net_volume_ml": 330}),
    )
    .await;

    // A second tenant sees none of tenant A's audit events.
    let (token_b, _tid_b) = app.register().await;
    let page: Value = app
        .get("/api/v1/compliance-audit", &token_b)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(page["total"].as_i64().unwrap(), 0);
}
