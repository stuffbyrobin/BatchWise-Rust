//! Integration tests for roles and route authorisation (Phase 15): each role
//! gets the agreed access, record-level rules need an Owner or Manager, and
//! deactivated members are refused.

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
        trust_proxy_headers: false,
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

const FLAGS: &str = r#"{"reporting":true,"duty":true,"sales":true,"labels":true,"packaging":true,"traceability":true,"tracking":true,"allergens":true,"equipment_maintenance":true}"#;
const PASSWORD: &str = "Sup3rSecret!pw";

/// Registers a user (with a throwaway tenant), moves them into `tenant_id` with
/// `role`, and logs in again so the token carries the new tenant. Returns the
/// access token and user id.
async fn member(app: &TestApp, tenant_id: Uuid, role: &str) -> (String, Uuid) {
    let email = format!("{role}-{}@example.com", uniq());
    let resp = app
        .client
        .post(format!("{}/api/v1/auth/register", app.base))
        .json(&json!({"email": email, "password": PASSWORD, "display_name": role, "tenant_name": format!("Temp {}", uniq())}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let user_id = Uuid::parse_str(
        resp.json::<Value>().await.unwrap()["user_id"]
            .as_str()
            .unwrap(),
    )
    .unwrap();

    let pool = sqlx::PgPool::connect(&app.db_url).await.unwrap();
    sqlx::query("UPDATE users SET tenant_id = $1, role = $2 WHERE id = $3")
        .bind(tenant_id)
        .bind(role)
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    let resp = app
        .client
        .post(format!("{}/api/v1/auth/login", app.base))
        .json(&json!({"email": email, "password": PASSWORD}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let token = resp.json::<Value>().await.unwrap()["access_token"]
        .as_str()
        .unwrap()
        .to_string();
    (token, user_id)
}

async fn call(
    app: &TestApp,
    method: reqwest::Method,
    path: &str,
    token: &str,
    body: Option<Value>,
) -> reqwest::Response {
    let mut req = app
        .client
        .request(method, format!("{}{path}", app.base))
        .bearer_auth(token);
    if let Some(body) = body {
        req = req.json(&body);
    }
    req.send().await.unwrap()
}

#[tokio::test]
async fn owners_see_their_role() {
    let app = spawn_app().await;
    let (token, _tid) = app.register().await;
    let me: Value = app
        .get("/api/v1/auth/me", &token)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(me["role"], json!("owner"));
}

#[tokio::test]
async fn each_role_gets_the_agreed_access() {
    use reqwest::Method as M;
    let app = spawn_app().await;
    let (owner, tid) = app.register().await;
    app.enable(tid, FLAGS).await;
    let (manager, _) = member(&app, tid, "manager").await;
    let (brewer, _) = member(&app, tid, "brewer").await;
    let (sales, _) = member(&app, tid, "sales").await;
    let (viewer, _) = member(&app, tid, "viewer").await;
    let tokens = [
        ("owner", &owner),
        ("manager", &manager),
        ("brewer", &brewer),
        ("sales", &sales),
        ("viewer", &viewer),
    ];

    // Whether each of owner, manager, brewer, sales and viewer may call the
    // route. Refused calls are 403; an allowed call may still fail validation.
    let all = [true; 5];
    let cases: Vec<(M, &str, Option<Value>, [bool; 5])> = vec![
        (M::GET, "/api/v1/recipes", None, all),
        (
            M::POST,
            "/api/v1/recipes",
            Some(json!({})),
            [true, true, true, false, false],
        ),
        (M::GET, "/api/v1/tenants/current", None, all),
        (
            M::PATCH,
            "/api/v1/tenants/current",
            Some(json!({"region": "South West"})),
            [true, false, false, false, false],
        ),
        (M::GET, "/api/v1/customers", None, all),
        (
            M::POST,
            "/api/v1/customers",
            Some(json!({})),
            [true, true, false, true, false],
        ),
        (
            M::POST,
            "/api/v1/distribution-movements",
            Some(json!({})),
            [true, true, true, true, false],
        ),
        (
            M::GET,
            "/api/v1/cost-rates",
            None,
            [true, true, true, false, true],
        ),
        (
            M::POST,
            "/api/v1/cost-rates",
            Some(json!({})),
            [true, true, false, false, false],
        ),
        (
            M::GET,
            "/api/v1/duty-returns",
            None,
            [true, true, false, false, true],
        ),
        (
            M::POST,
            "/api/v1/duty-returns/compile",
            Some(json!({})),
            [true, true, false, false, false],
        ),
        (
            M::GET,
            "/api/v1/compliance-audit",
            None,
            [true, true, false, false, true],
        ),
        (
            M::POST,
            "/api/v1/water-adjustments/calculate",
            Some(json!({})),
            all,
        ),
        (M::GET, "/api/v1/dashboard/stats", None, all),
    ];

    let mut wrong = Vec::new();
    for (method, path, body, allowed) in &cases {
        for ((name, token), allowed) in tokens.iter().zip(allowed) {
            let status = call(&app, method.clone(), path, token, body.clone())
                .await
                .status();
            if (status != 403) != *allowed {
                wrong.push(format!("{name} {method} {path}: {status}"));
            }
        }
    }
    assert!(wrong.is_empty(), "unexpected access:\n{}", wrong.join("\n"));
}

#[tokio::test]
async fn only_owners_and_managers_spoil_a_completed_batch() {
    let app = spawn_app().await;
    let (owner, tid) = app.register().await;
    let (brewer, _) = member(&app, tid, "brewer").await;
    let (manager, _) = member(&app, tid, "manager").await;

    // No ingredients, so brewing deducts no stock.
    let recipe: Value = app
        .post("/api/v1/recipes", &owner, json!({"name": format!("Role Ale {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0}))
        .await
        .json()
        .await
        .unwrap();
    let batch: Value = app
        .post("/api/v1/batches", &owner, json!({"recipe_id": recipe["id"], "batch_number": format!("B-{}", uniq()), "name": "Role Batch"}))
        .await
        .json()
        .await
        .unwrap();
    let id = batch["batch"]["id"].as_str().unwrap();
    let path = format!("/api/v1/batches/{id}/transition");
    for to in [
        "brewing",
        "fermenting",
        "conditioning",
        "packaging",
        "completed",
    ] {
        let resp = app.post(&path, &brewer, json!({"to_status": to})).await;
        assert_eq!(resp.status(), 200, "{to}");
    }

    let spoil = json!({"to_status": "spoiled", "reason": "Infected"});
    let resp = app.post(&path, &brewer, spoil.clone()).await;
    assert_eq!(resp.status(), 403);
    let resp = app.post(&path, &manager, spoil).await;
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn only_owners_and_managers_approve_label_records() {
    let app = spawn_app().await;
    let (owner, tid) = app.register().await;
    app.enable(tid, FLAGS).await;
    let (brewer, _) = member(&app, tid, "brewer").await;
    let (manager, _) = member(&app, tid, "manager").await;
    let batch = app.make_batch(&owner).await;

    let resp = app
        .post(
            "/api/v1/label-records",
            &brewer,
            json!({"batch_id": batch, "net_volume_ml": 500}),
        )
        .await;
    assert_eq!(resp.status(), 201);
    let id = resp.json::<Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let path = format!("/api/v1/label-records/{id}");
    let approve = Some(json!({"status": "approved"}));

    let resp = call(
        &app,
        reqwest::Method::PATCH,
        &path,
        &brewer,
        approve.clone(),
    )
    .await;
    assert_eq!(resp.status(), 403);
    // The manager may approve; the record itself may still lack mandatory fields.
    let resp = call(&app, reqwest::Method::PATCH, &path, &manager, approve).await;
    assert_ne!(resp.status(), 403);
}

#[tokio::test]
async fn deactivated_members_are_refused() {
    let app = spawn_app().await;
    let (_owner, tid) = app.register().await;
    let (brewer, brewer_id) = member(&app, tid, "brewer").await;

    let pool = sqlx::PgPool::connect(&app.db_url).await.unwrap();
    sqlx::query("UPDATE users SET is_active = false WHERE id = $1")
        .bind(brewer_id)
        .execute(&pool)
        .await
        .unwrap();

    let resp = app.get("/api/v1/recipes", &brewer).await;
    assert_eq!(resp.status(), 401);
}
