//! Integration tests for label design (brand-assets, brand-profiles,
//! label-designs; `label_design`-feature-gated).

use std::net::SocketAddr;

use base64::Engine;
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
            "email": format!("ld-{}@example.com", uniq()),
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

    /// Creates a batch with an approved label record; returns the batch id.
    async fn make_approved_batch(&self, token: &str) -> String {
        let recipe: Value = self
            .post(
                "/api/v1/recipes",
                token,
                json!({"name": format!("Design Ale {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0, "yeasts": [{"name": "US-05", "amount": 11.0, "unit": "g", "attenuation_pct": 80.0}], "fermentables": [{"step_order": 1, "name": "Pale", "amount": 5.0, "unit": "kg", "potential_ppg": 37.0}]}),
            )
            .await
            .json()
            .await
            .unwrap();
        let batch: Value = self
            .post(
                "/api/v1/batches",
                token,
                json!({"recipe_id": recipe["id"], "batch_number": format!("B-{}", uniq()), "name": "Design Batch"}),
            )
            .await
            .json()
            .await
            .unwrap();
        let batch_id = batch["batch"]["id"].as_str().unwrap().to_string();

        let rec: Value = self
            .post(
                "/api/v1/label-records",
                token,
                json!({"batch_id": batch_id, "net_volume_ml": 500}),
            )
            .await
            .json()
            .await
            .unwrap();
        let rec_id = rec["id"].as_str().unwrap();
        let resp = self
            .patch(
                &format!("/api/v1/label-records/{rec_id}"),
                token,
                json!({"status": "approved"}),
            )
            .await;
        assert_eq!(resp.status(), 200, "approve label record");
        batch_id
    }
}

// Known-good 1x1 transparent PNG.
fn png_1x1() -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M8AAAMBAQDJ/pLvAAAAAElFTkSuQmCC")
        .unwrap()
}

#[tokio::test]
async fn label_design_feature_gate_blocks_home_tenant() {
    let app = spawn_app().await;
    let (token, _tid) = app.register().await;
    let resp = app.get("/api/v1/label-designs", &token).await;
    assert_eq!(resp.status(), 403);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["details"]["required_feature"],
        json!("label_design")
    );
}

#[tokio::test]
async fn brand_profile_crud() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable(tid, "{\"label_design\":true}").await;

    // Create with defaults applied.
    let resp = app
        .post(
            "/api/v1/brand-profiles",
            &token,
            json!({"name": format!("House {}", uniq())}),
        )
        .await;
    assert_eq!(resp.status(), 201);
    let p: Value = resp.json().await.unwrap();
    let pid = p["id"].as_str().unwrap().to_string();
    assert_eq!(p["primary_color"], json!("#000000"));
    assert_eq!(p["secondary_color"], json!("#ffffff"));
    assert_eq!(p["font_family"], json!("helvetica"));

    // List ({"items": [...]}).
    let list: Value = app
        .get("/api/v1/brand-profiles", &token)
        .await
        .json()
        .await
        .unwrap();
    assert!(!list["items"].as_array().unwrap().is_empty());

    // Get.
    assert_eq!(
        app.get(&format!("/api/v1/brand-profiles/{pid}"), &token)
            .await
            .status(),
        200
    );

    // Patch.
    let resp = app
        .patch(
            &format!("/api/v1/brand-profiles/{pid}"),
            &token,
            json!({"primary_color": "#ff0000"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["primary_color"],
        json!("#ff0000")
    );

    // Invalid colour → 400.
    let resp = app
        .post(
            "/api/v1/brand-profiles",
            &token,
            json!({"name": format!("Bad {}", uniq()), "primary_color": "#zzz"}),
        )
        .await;
    assert_eq!(resp.status(), 400);

    // Delete → 204.
    assert_eq!(
        app.delete(&format!("/api/v1/brand-profiles/{pid}"), &token)
            .await
            .status(),
        204
    );
}

#[tokio::test]
async fn brand_asset_upload_and_fetch() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable(tid, "{\"label_design\":true}").await;

    let bytes = png_1x1();
    let part = reqwest::multipart::Part::bytes(bytes.clone())
        .file_name("logo.png")
        .mime_str("image/png")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);
    let resp = app
        .client
        .post(format!("{}/api/v1/brand-assets", app.base))
        .bearer_auth(&token)
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "upload asset");
    let a: Value = resp.json().await.unwrap();
    let aid = a["id"].as_str().unwrap().to_string();
    assert!(a["byte_size"].as_i64().unwrap() > 0);

    // Fetch binary.
    let resp = app
        .get(&format!("/api/v1/brand-assets/{aid}"), &token)
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "image/png"
    );
    let fetched = resp.bytes().await.unwrap();
    assert_eq!(fetched.as_ref(), bytes.as_slice());

    // Non-image content-type → 415.
    let part = reqwest::multipart::Part::bytes(b"hello".to_vec())
        .file_name("x.txt")
        .mime_str("text/plain")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);
    let resp = app
        .client
        .post(format!("{}/api/v1/brand-assets", app.base))
        .bearer_auth(&token)
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 415);

    // Delete → 204.
    assert_eq!(
        app.delete(&format!("/api/v1/brand-assets/{aid}"), &token)
            .await
            .status(),
        204
    );
}

#[tokio::test]
async fn design_lifecycle_and_render() {
    let app = spawn_app().await;
    let (token, tid) = app.register().await;
    app.enable(tid, "{\"label_design\":true,\"labels\":true}")
        .await;

    let batch_id = app.make_approved_batch(&token).await;

    // Create a bottle design.
    let resp = app
        .post(
            "/api/v1/label-designs",
            &token,
            json!({
                "kind": "bottle",
                "name": "My Bottle",
                "batch_id": batch_id,
                "size_key": "bottle_front_90x120",
                "template_key": "compliance_standard"
            }),
        )
        .await;
    assert_eq!(resp.status(), 201, "create bottle design");
    let d: Value = resp.json().await.unwrap();
    let did = d["id"].as_str().unwrap().to_string();

    // Render model.
    let resp = app
        .get(&format!("/api/v1/label-designs/{did}/render"), &token)
        .await;
    assert_eq!(resp.status(), 200);
    let model: Value = resp.json().await.unwrap();
    assert_eq!(model["width_mm"].as_f64().unwrap(), 90.0);
    assert!(model["fields"]["product_name"]
        .as_str()
        .is_some_and(|s| !s.is_empty()));

    // Render PDF.
    let resp = app
        .get(&format!("/api/v1/label-designs/{did}/render.pdf"), &token)
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/pdf"
    );
    let pdf = resp.bytes().await.unwrap();
    assert!(pdf.starts_with(b"%PDF"), "PDF magic bytes");

    // Bottle design with a recipe_id (no batch) → 422 kind_source_mismatch.
    let resp = app
        .post(
            "/api/v1/label-designs",
            &token,
            json!({
                "kind": "bottle",
                "name": "Bad",
                "recipe_id": Uuid::new_v4().to_string(),
                "size_key": "bottle_front_90x120",
                "template_key": "compliance_standard"
            }),
        )
        .await;
    assert_eq!(resp.status(), 422);
    assert_eq!(
        resp.json::<Value>().await.unwrap()["details"]["rule"],
        json!("kind_source_mismatch")
    );

    // Wrong size for kind → 400.
    let resp = app
        .post(
            "/api/v1/label-designs",
            &token,
            json!({
                "kind": "bottle",
                "name": "Bad Size",
                "batch_id": batch_id,
                "size_key": "lens_round_100",
                "template_key": "compliance_standard"
            }),
        )
        .await;
    assert_eq!(resp.status(), 400);
}
