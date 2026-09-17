//! Integration tests for tenant members (`/members`, Phase 15): who may list
//! and change members, changes applying at once, and the last-Owner guard.

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

const PASSWORD: &str = "Sup3rSecret!pw";

struct Member {
    token: String,
    refresh: String,
    id: Uuid,
    email: String,
}

/// Registers a user (with a throwaway tenant), moves them into `tenant_id` with
/// `role`, and logs in so the tokens carry the new tenant.
async fn member(app: &TestApp, tenant_id: Uuid, role: &str) -> Member {
    let email = format!("{role}-{}@example.com", uniq());
    let resp = app
        .client
        .post(format!("{}/api/v1/auth/register", app.base))
        .json(&json!({"email": email, "password": PASSWORD, "display_name": role, "tenant_name": format!("Temp {}", uniq())}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let id = Uuid::parse_str(
        resp.json::<Value>().await.unwrap()["user_id"]
            .as_str()
            .unwrap(),
    )
    .unwrap();

    let pool = sqlx::PgPool::connect(&app.db_url).await.unwrap();
    sqlx::query("UPDATE users SET tenant_id = $1, role = $2 WHERE id = $3")
        .bind(tenant_id)
        .bind(role)
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    let resp = login(app, &email).await;
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    Member {
        token: body["access_token"].as_str().unwrap().to_string(),
        refresh: body["refresh_token"].as_str().unwrap().to_string(),
        id,
        email,
    }
}

async fn login(app: &TestApp, email: &str) -> reqwest::Response {
    app.client
        .post(format!("{}/api/v1/auth/login", app.base))
        .json(&json!({"email": email, "password": PASSWORD}))
        .send()
        .await
        .unwrap()
}

async fn patch(app: &TestApp, path: &str, token: &str, body: Value) -> reqwest::Response {
    app.client
        .patch(format!("{}{path}", app.base))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .unwrap()
}

async fn my_id(app: &TestApp, token: &str) -> Uuid {
    let me: Value = app
        .get("/api/v1/auth/me", token)
        .await
        .json()
        .await
        .unwrap();
    Uuid::parse_str(me["user_id"].as_str().unwrap()).unwrap()
}

fn recipe() -> Value {
    json!({"name": format!("Member Ale {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0})
}

#[tokio::test]
async fn owners_and_managers_list_members() {
    let app = spawn_app().await;
    let (owner, tid) = app.register().await;
    let manager = member(&app, tid, "manager").await;
    let brewer = member(&app, tid, "brewer").await;

    let resp = app.get("/api/v1/members", &owner).await;
    assert_eq!(resp.status(), 200);
    let list: Value = resp.json().await.unwrap();
    let items = list["items"].as_array().unwrap();
    let roles: Vec<&str> = items.iter().map(|m| m["role"].as_str().unwrap()).collect();
    assert_eq!(roles, ["owner", "manager", "brewer"]);
    assert!(items.iter().all(|m| m.get("password_hash").is_none()));

    assert_eq!(
        app.get("/api/v1/members", &manager.token).await.status(),
        200
    );
    assert_eq!(
        app.get("/api/v1/members", &brewer.token).await.status(),
        403
    );
}

#[tokio::test]
async fn managers_manage_only_brewer_sales_and_viewer_members() {
    let app = spawn_app().await;
    let (owner, tid) = app.register().await;
    let owner_id = my_id(&app, &owner).await;
    let manager = member(&app, tid, "manager").await;
    let other_manager = member(&app, tid, "manager").await;
    let brewer = member(&app, tid, "brewer").await;
    let brewer_path = format!("/api/v1/members/{}", brewer.id);

    let resp = patch(&app, &brewer_path, &manager.token, json!({"role": "sales"})).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<Value>().await.unwrap()["role"], json!("sales"));

    let refused = [
        (brewer_path.clone(), json!({"role": "manager"})),
        (
            format!("/api/v1/members/{}", other_manager.id),
            json!({"role": "viewer"}),
        ),
        (
            format!("/api/v1/members/{owner_id}"),
            json!({"is_active": false}),
        ),
    ];
    for (path, body) in refused {
        assert_eq!(
            patch(&app, &path, &manager.token, body).await.status(),
            403,
            "{path}"
        );
    }
    let resp = patch(&app, &brewer_path, &manager.token, json!({"role": "admin"})).await;
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn role_changes_apply_at_once_and_are_audited() {
    let app = spawn_app().await;
    let (owner, tid) = app.register().await;
    let brewer = member(&app, tid, "brewer").await;

    assert_eq!(
        app.post("/api/v1/recipes", &brewer.token, recipe())
            .await
            .status(),
        201
    );
    let path = format!("/api/v1/members/{}", brewer.id);
    assert_eq!(
        patch(&app, &path, &owner, json!({"role": "viewer"}))
            .await
            .status(),
        200
    );
    // The same access token is now a viewer's.
    assert_eq!(
        app.post("/api/v1/recipes", &brewer.token, recipe())
            .await
            .status(),
        403
    );

    let pool = sqlx::PgPool::connect(&app.db_url).await.unwrap();
    let data: Value = sqlx::query_scalar(
        "SELECT event_data FROM compliance_audit_log \
         WHERE entity_id = $1 AND event_type = 'member.role_changed'",
    )
    .bind(brewer.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(data["from_role"], json!("brewer"));
    assert_eq!(data["to_role"], json!("viewer"));
    assert_eq!(data["email"], json!(brewer.email));
}

#[tokio::test]
async fn deactivated_members_lose_access_until_reactivated() {
    let app = spawn_app().await;
    let (owner, tid) = app.register().await;
    let brewer = member(&app, tid, "brewer").await;
    let path = format!("/api/v1/members/{}", brewer.id);

    assert_eq!(
        app.get("/api/v1/recipes", &brewer.token).await.status(),
        200
    );
    assert_eq!(
        patch(&app, &path, &owner, json!({"is_active": false}))
            .await
            .status(),
        200
    );

    assert_eq!(
        app.get("/api/v1/recipes", &brewer.token).await.status(),
        401
    );
    let resp = app
        .client
        .post(format!("{}/api/v1/auth/refresh", app.base))
        .json(&json!({"refresh_token": brewer.refresh}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    assert_ne!(login(&app, &brewer.email).await.status(), 200);

    assert_eq!(
        patch(&app, &path, &owner, json!({"is_active": true}))
            .await
            .status(),
        200
    );
    // Tokens from before the deactivation stay revoked; signing in again works.
    assert_eq!(
        app.get("/api/v1/recipes", &brewer.token).await.status(),
        401
    );
    let resp = login(&app, &brewer.email).await;
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        app.get("/api/v1/recipes", body["access_token"].as_str().unwrap())
            .await
            .status(),
        200
    );
}

#[tokio::test]
async fn a_tenant_always_keeps_an_active_owner() {
    let app = spawn_app().await;
    let (owner, tid) = app.register().await;
    let owner_path = format!("/api/v1/members/{}", my_id(&app, &owner).await);
    let manager = member(&app, tid, "manager").await;

    for body in [json!({"role": "manager"}), json!({"is_active": false})] {
        let resp = patch(&app, &owner_path, &owner, body).await;
        assert_eq!(resp.status(), 422);
        let err: Value = resp.json().await.unwrap();
        assert_eq!(err["details"]["rule"], json!("last_owner"));
    }
    let resp = app
        .client
        .delete(format!("{}/api/v1/auth/me", app.base))
        .bearer_auth(&owner)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 422);

    // With a second Owner, the first may step down.
    let manager_path = format!("/api/v1/members/{}", manager.id);
    assert_eq!(
        patch(&app, &manager_path, &owner, json!({"role": "owner"}))
            .await
            .status(),
        200
    );
    assert_eq!(
        patch(&app, &owner_path, &owner, json!({"role": "manager"}))
            .await
            .status(),
        200
    );
}

#[tokio::test]
async fn members_of_other_tenants_are_not_found() {
    let app = spawn_app().await;
    let (owner_a, _tenant_a) = app.register().await;
    let (_owner_b, tenant_b) = app.register().await;
    let brewer_b = member(&app, tenant_b, "brewer").await;

    let path = format!("/api/v1/members/{}", brewer_b.id);
    assert_eq!(
        patch(&app, &path, &owner_a, json!({"role": "viewer"}))
            .await
            .status(),
        404
    );
}
