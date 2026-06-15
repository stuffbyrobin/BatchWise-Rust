//! Integration tests for procurement (suppliers + purchase orders, `procurement`
//! feature).

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
            "email": format!("p-{}@example.com", uniq()),
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

    async fn create_supplier(&self, token: &str) -> Value {
        let resp = self
            .post(
                "/api/v1/suppliers",
                token,
                json!({ "name": format!("Supplier {}", uniq()) }),
            )
            .await;
        assert_eq!(resp.status(), 201, "create supplier");
        resp.json().await.unwrap()
    }

    async fn create_po(&self, token: &str, supplier_id: &str) -> Value {
        let resp = self
            .post(
                "/api/v1/purchase-orders",
                token,
                json!({ "supplier_id": supplier_id }),
            )
            .await;
        assert_eq!(resp.status(), 201, "create po");
        resp.json().await.unwrap()
    }

    async fn add_line(&self, token: &str, po_id: &str, qty: f64) -> Value {
        let resp = self
            .post(
                &format!("/api/v1/purchase-orders/{po_id}/lines"),
                token,
                json!({
                    "ingredient_type": "fermentable",
                    "ingredient_name": "Maris Otter",
                    "quantity": qty,
                    "unit": "kg",
                    "unit_cost_pence": 150
                }),
            )
            .await;
        assert_eq!(resp.status(), 201, "add line");
        resp.json().await.unwrap()
    }
}

#[tokio::test]
async fn procurement_feature_gate_blocks_home_tenant() {
    let app = spawn_app().await;
    let (token, _tid) = app.register().await;
    let resp = app.get("/api/v1/suppliers", &token).await;
    assert_eq!(resp.status(), 403);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["details"]["required_feature"],
        json!("procurement")
    );
}

#[tokio::test]
async fn supplier_crud_and_delete_guard() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable(tid, "{\"procurement\":true}").await;

    let name = format!("Acme Malt {}", uniq());
    let resp = app
        .post(
            "/api/v1/suppliers",
            &token,
            json!({ "name": name, "contact_name": "Jo", "email": "jo@acme.test" }),
        )
        .await;
    assert_eq!(resp.status(), 201);
    assert!(resp
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("/api/v1/suppliers/"));
    let sup: Value = resp.json().await.unwrap();
    let sid = sup["id"].as_str().unwrap().to_string();
    assert_eq!(sup["contact_name"], json!("Jo"));

    // Get.
    let got: Value = app
        .get(&format!("/api/v1/suppliers/{sid}"), &token)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(got["id"], json!(sid));

    // List with search.
    let list: Value = app
        .get(&format!("/api/v1/suppliers?search={}", &name[..4]), &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(list["total"].as_i64().unwrap() >= 1);

    // Patch name + clear contact_name via explicit null.
    let new_name = format!("Acme Renamed {}", uniq());
    let patched: Value = app
        .patch(
            &format!("/api/v1/suppliers/{sid}"),
            &token,
            json!({ "name": new_name, "contact_name": null }),
        )
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(patched["name"], json!(new_name));
    assert_eq!(patched["contact_name"], json!(null));
    // Email left untouched (field omitted).
    assert_eq!(patched["email"], json!("jo@acme.test"));

    // Duplicate-name create → 409.
    let dup = app
        .post("/api/v1/suppliers", &token, json!({ "name": new_name }))
        .await;
    assert_eq!(dup.status(), 409);

    // Create a PO for the supplier, then deleting the supplier → 422.
    let _po = app.create_po(&token, &sid).await;
    let del = app
        .delete(&format!("/api/v1/suppliers/{sid}"), &token)
        .await;
    assert_eq!(del.status(), 422);
    assert_eq!(
        del.json::<Value>().await.unwrap()["details"]["rule"],
        json!("supplier_has_orders")
    );
}

#[tokio::test]
async fn po_lifecycle_and_receive() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable(tid, "{\"procurement\":true}").await;

    let sup = app.create_supplier(&token).await;
    let sid = sup["id"].as_str().unwrap().to_string();

    // First PO of a fresh tenant gets PO-00001.
    let po = app.create_po(&token, &sid).await;
    let pid = po["id"].as_str().unwrap().to_string();
    assert_eq!(po["po_number"], json!("PO-00001"));
    assert_eq!(po["status"], json!("draft"));
    assert!(po["lines"].as_array().unwrap().is_empty());

    // A second, empty PO cannot be marked sent (no_lines).
    let po2 = app.create_po(&token, &sid).await;
    let pid2 = po2["id"].as_str().unwrap().to_string();
    let r = app
        .patch(
            &format!("/api/v1/purchase-orders/{pid2}"),
            &token,
            json!({ "status": "sent" }),
        )
        .await;
    assert_eq!(r.status(), 422);
    assert_eq!(
        r.json::<Value>().await.unwrap()["details"]["rule"],
        json!("no_lines")
    );

    // Add two lines to the first PO.
    let l1 = app.add_line(&token, &pid, 10.0).await;
    let l2 = app.add_line(&token, &pid, 5.0).await;
    let l1id = l1["id"].as_str().unwrap().to_string();
    let l2id = l2["id"].as_str().unwrap().to_string();
    assert_eq!(l1["unit_cost_currency"], json!("GBP"));

    // Mark sent.
    let r = app
        .patch(
            &format!("/api/v1/purchase-orders/{pid}"),
            &token,
            json!({ "status": "sent" }),
        )
        .await;
    assert_eq!(r.status(), 200);
    assert_eq!(r.json::<Value>().await.unwrap()["status"], json!("sent"));

    // Receive line 1 fully, line 2 not at all → partially_received.
    let r = app
        .post(
            &format!("/api/v1/purchase-orders/{pid}/receive"),
            &token,
            json!({ "lines": [{ "line_id": l1id, "received_quantity": 10.0 }] }),
        )
        .await;
    assert_eq!(r.status(), 200);
    assert_eq!(
        r.json::<Value>().await.unwrap()["status"],
        json!("partially_received")
    );

    // Receive remaining → received.
    let r = app
        .post(
            &format!("/api/v1/purchase-orders/{pid}/receive"),
            &token,
            json!({ "lines": [{ "line_id": l2id, "received_quantity": 5.0 }] }),
        )
        .await;
    assert_eq!(r.status(), 200);
    assert_eq!(
        r.json::<Value>().await.unwrap()["status"],
        json!("received")
    );

    // A received PO's lines cannot be edited.
    let r = app
        .patch(
            &format!("/api/v1/purchase-orders/{pid}/lines/{l1id}"),
            &token,
            json!({ "quantity": 99.0 }),
        )
        .await;
    assert_eq!(r.status(), 422);
    assert_eq!(
        r.json::<Value>().await.unwrap()["details"]["rule"],
        json!("po_not_draft")
    );

    // Deleting a non-draft PO → 422.
    let r = app
        .delete(&format!("/api/v1/purchase-orders/{pid}"), &token)
        .await;
    assert_eq!(r.status(), 422);
    assert_eq!(
        r.json::<Value>().await.unwrap()["details"]["rule"],
        json!("po_not_draft")
    );

    // Deleting a draft PO works.
    let r = app
        .delete(&format!("/api/v1/purchase-orders/{pid2}"), &token)
        .await;
    assert_eq!(r.status(), 204);
}
