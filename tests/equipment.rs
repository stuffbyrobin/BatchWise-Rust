//! Integration tests for the equipment maintenance module (`equipment` +
//! `maintenance-due`, `equipment_maintenance` feature).

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
            "email": format!("e-{}@example.com", uniq()),
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

    async fn create_equipment(&self, token: &str) -> Value {
        let resp = self
            .post(
                "/api/v1/equipment",
                token,
                json!({ "name": format!("Fermenter {}", uniq()), "equipment_type": "fermenter" }),
            )
            .await;
        assert_eq!(resp.status(), 201, "create equipment");
        resp.json().await.unwrap()
    }
}

#[tokio::test]
async fn equipment_feature_gate() {
    // `equipment_maintenance` is a pro-tier flag; the home-tier default tenant
    // does not have it, so the gate returns 403 without disabling anything.
    let app = spawn_app().await;
    let (token, _tid) = app.register().await;
    let resp = app.get("/api/v1/equipment", &token).await;
    assert_eq!(resp.status(), 403);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["details"]["required_feature"],
        json!("equipment_maintenance")
    );
}

#[tokio::test]
async fn equipment_crud() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable(tid, "{\"equipment_maintenance\":true}").await;

    // Create.
    let resp = app
        .post(
            "/api/v1/equipment",
            &token,
            json!({ "name": "Boil Kettle", "equipment_type": "kettle", "serial_number": "BK-1" }),
        )
        .await;
    assert_eq!(resp.status(), 201);
    assert!(resp
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("/api/v1/equipment/"));
    let eq: Value = resp.json().await.unwrap();
    let eid = eq["id"].as_str().unwrap().to_string();
    assert_eq!(eq["status"], json!("active"));
    assert_eq!(eq["overdue_schedule_count"], json!(0));
    // Lifetime cost present on create (a single-equipment read), value 0.
    assert_eq!(eq["lifetime_maintenance_cost_pence"], json!(0));

    // Get.
    let got: Value = app
        .get(&format!("/api/v1/equipment/{eid}"), &token)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(got["id"], json!(eid));
    assert_eq!(got["lifetime_maintenance_cost_pence"], json!(0));

    // List: lifetime cost field ABSENT from list items.
    let list: Value = app
        .get("/api/v1/equipment", &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(list["total"].as_i64().unwrap() >= 1);
    let item = &list["items"][0];
    assert!(
        item.get("lifetime_maintenance_cost_pence").is_none(),
        "lifetime cost must be omitted from list items"
    );

    // Patch status retired.
    let patched: Value = app
        .patch(
            &format!("/api/v1/equipment/{eid}"),
            &token,
            json!({ "status": "retired" }),
        )
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(patched["status"], json!("retired"));

    // Delete.
    let del = app
        .delete(&format!("/api/v1/equipment/{eid}"), &token)
        .await;
    assert_eq!(del.status(), 204);
}

#[tokio::test]
async fn schedule_and_due() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable(tid, "{\"equipment_maintenance\":true}").await;

    let eq = app.create_equipment(&token).await;
    let eid = eq["id"].as_str().unwrap().to_string();

    let thirty_days_ago = (chrono::Utc::now() - chrono::Duration::days(30)).to_rfc3339();
    let resp = app
        .post(
            &format!("/api/v1/equipment/{eid}/schedules"),
            &token,
            json!({
                "task_name": "Calibrate sensor",
                "interval_days": 7,
                "last_performed_at": thirty_days_ago,
            }),
        )
        .await;
    assert_eq!(resp.status(), 201);
    let sched: Value = resp.json().await.unwrap();
    assert_eq!(sched["is_overdue"], json!(true));
    assert!(sched["days_until_due"].as_i64().unwrap() < 0);

    // Overdue-only feed includes the schedule.
    let due: Value = app
        .get("/api/v1/maintenance-due?overdue_only=true", &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(due["total"].as_i64().unwrap() >= 1);
    let sid = sched["id"].as_str().unwrap();
    assert!(due["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|it| it["schedule_id"] == json!(sid)));

    // Default window also includes it (overdue items are <= now).
    let due_default: Value = app
        .get("/api/v1/maintenance-due", &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(due_default["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|it| it["schedule_id"] == json!(sid)));
}

#[tokio::test]
async fn event_advances_schedule_and_mismatch() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable(tid, "{\"equipment_maintenance\":true}").await;

    // Equipment A + an overdue schedule S.
    let eq_a = app.create_equipment(&token).await;
    let a_id = eq_a["id"].as_str().unwrap().to_string();
    let thirty_days_ago = (chrono::Utc::now() - chrono::Duration::days(30)).to_rfc3339();
    let sched: Value = app
        .post(
            &format!("/api/v1/equipment/{a_id}/schedules"),
            &token,
            json!({
                "task_name": "Service pump",
                "interval_days": 7,
                "last_performed_at": thirty_days_ago,
            }),
        )
        .await
        .json()
        .await
        .unwrap();
    let s_id = sched["id"].as_str().unwrap().to_string();
    assert_eq!(sched["is_overdue"], json!(true));

    // Post an event against S → 201.
    let ev_resp = app
        .post(
            &format!("/api/v1/equipment/{a_id}/events"),
            &token,
            json!({ "schedule_id": s_id, "event_type": "service", "cost_pence": 1500 }),
        )
        .await;
    assert_eq!(ev_resp.status(), 201);
    let event: Value = ev_resp.json().await.unwrap();
    let ev_id = event["id"].as_str().unwrap().to_string();

    // Equipment A's lifetime cost becomes 1500.
    let got_a: Value = app
        .get(&format!("/api/v1/equipment/{a_id}"), &token)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(got_a["lifetime_maintenance_cost_pence"], json!(1500));

    // Schedule advanced: last_performed_at now recent → no longer overdue.
    let schedules: Value = app
        .get(&format!("/api/v1/equipment/{a_id}/schedules"), &token)
        .await
        .json()
        .await
        .unwrap();
    let s = schedules["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|it| it["id"] == json!(s_id))
        .unwrap();
    assert_eq!(s["is_overdue"], json!(false));

    // Equipment B; posting an event on B referencing S → 422 mismatch.
    let eq_b = app.create_equipment(&token).await;
    let b_id = eq_b["id"].as_str().unwrap().to_string();
    let mismatch = app
        .post(
            &format!("/api/v1/equipment/{b_id}/events"),
            &token,
            json!({ "schedule_id": s_id, "event_type": "service" }),
        )
        .await;
    assert_eq!(mismatch.status(), 422);
    assert_eq!(
        mismatch.json::<Value>().await.unwrap()["details"]["rule"],
        json!("schedule_equipment_mismatch")
    );

    // Delete the event → 204.
    let del = app
        .delete(&format!("/api/v1/equipment/{a_id}/events/{ev_id}"), &token)
        .await;
    assert_eq!(del.status(), 204);
}
