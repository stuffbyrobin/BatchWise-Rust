//! Integration tests for member invitations: creating, previewing, accepting,
//! expiry, revocation, and who may invite whom.

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

fn new_email() -> String {
    format!("invitee-{}@example.com", uniq())
}

async fn invite(app: &TestApp, token: &str, email: &str, role: &str) -> reqwest::Response {
    app.post(
        "/api/v1/members/invitations",
        token,
        json!({"email": email, "role": role}),
    )
    .await
}

async fn public_post(app: &TestApp, path: &str, body: Value) -> reqwest::Response {
    app.client
        .post(format!("{}{path}", app.base))
        .json(&body)
        .send()
        .await
        .unwrap()
}

async fn preview(app: &TestApp, token: &str) -> reqwest::Response {
    public_post(app, "/api/v1/auth/invitation", json!({"token": token})).await
}

async fn accept(app: &TestApp, token: &str) -> reqwest::Response {
    public_post(
        app,
        "/api/v1/auth/accept-invitation",
        json!({"token": token, "password": PASSWORD, "display_name": "Invitee"}),
    )
    .await
}

/// Creates an invitation and returns (invitation id, one-time token).
async fn invited(app: &TestApp, token: &str, email: &str, role: &str) -> (String, String) {
    let resp = invite(app, token, email, role).await;
    assert_eq!(resp.status(), 201);
    let body: Value = resp.json().await.unwrap();
    (
        body["id"].as_str().unwrap().to_string(),
        body["token"].as_str().unwrap().to_string(),
    )
}

/// Registers a user (with a throwaway tenant), moves them into `tenant_id` with
/// `role`, and logs in again. Returns the access token.
async fn member(app: &TestApp, tenant_id: Uuid, role: &str) -> String {
    let email = new_email();
    let resp = public_post(
        app,
        "/api/v1/auth/register",
        json!({"email": email, "password": PASSWORD, "display_name": role, "tenant_name": format!("Temp {}", uniq())}),
    )
    .await;
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
    let resp = public_post(
        app,
        "/api/v1/auth/login",
        json!({"email": email, "password": PASSWORD}),
    )
    .await;
    assert_eq!(resp.status(), 200);
    resp.json::<Value>().await.unwrap()["access_token"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn an_invitee_joins_with_the_invited_role() {
    let app = spawn_app().await;
    let (owner, tenant_id) = app.register().await;
    let email = new_email();
    let (_id, token) = invited(&app, &owner, &email, "brewer").await;

    // Listed while open, without the token.
    let list: Value = app
        .get("/api/v1/members/invitations", &owner)
        .await
        .json()
        .await
        .unwrap();
    let items = list["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["email"], json!(email));
    assert!(items[0].get("token").is_none());

    let resp = preview(&app, &token).await;
    assert_eq!(resp.status(), 200);
    let shown: Value = resp.json().await.unwrap();
    assert_eq!(shown["role"], json!("brewer"));
    assert_eq!(shown["email"], json!(email));
    assert!(shown["tenant_name"].as_str().is_some_and(|n| !n.is_empty()));

    // A weak password is refused and leaves the invitation usable.
    let resp = public_post(
        &app,
        "/api/v1/auth/accept-invitation",
        json!({"token": token, "password": "short", "display_name": "Invitee"}),
    )
    .await;
    assert!(resp.status().is_client_error());
    assert_ne!(resp.status(), 404);

    let resp = accept(&app, &token).await;
    assert_eq!(resp.status(), 201);
    let auth: Value = resp.json().await.unwrap();
    assert_eq!(auth["tenant_id"], json!(tenant_id.to_string()));
    assert_eq!(auth["role"], json!("brewer"));
    let access = auth["access_token"].as_str().unwrap();
    let recipe = json!({"name": format!("Joined {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0});
    assert_eq!(
        app.post("/api/v1/recipes", access, recipe).await.status(),
        201
    );

    // The link works once and the invitation is no longer open.
    assert_eq!(accept(&app, &token).await.status(), 404);
    assert_eq!(preview(&app, &token).await.status(), 404);
    let list: Value = app
        .get("/api/v1/members/invitations", &owner)
        .await
        .json()
        .await
        .unwrap();
    assert!(list["items"].as_array().unwrap().is_empty());
    let members: Value = app
        .get("/api/v1/members", &owner)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(members["items"].as_array().unwrap().len(), 2);

    // Inviting and joining are audited.
    let pool = sqlx::PgPool::connect(&app.db_url).await.unwrap();
    let events: Vec<String> = sqlx::query_scalar(
        "SELECT event_type FROM compliance_audit_log \
         WHERE tenant_id = $1 AND event_type LIKE 'member.%' ORDER BY created_at",
    )
    .bind(tenant_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(events, ["member.invited", "member.joined"]);
}

#[tokio::test]
async fn expired_and_revoked_invitations_cannot_be_used() {
    let app = spawn_app().await;
    let (owner, _tenant_id) = app.register().await;
    let expired_email = new_email();
    let (expired_id, expired_token) = invited(&app, &owner, &expired_email, "brewer").await;
    let (revoked_id, revoked_token) = invited(&app, &owner, &new_email(), "viewer").await;

    let pool = sqlx::PgPool::connect(&app.db_url).await.unwrap();
    sqlx::query("UPDATE invitations SET expires_at = now() - interval '1 minute' WHERE id = $1")
        .bind(Uuid::parse_str(&expired_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    for resp in [
        preview(&app, &expired_token).await,
        accept(&app, &expired_token).await,
    ] {
        assert_eq!(resp.status(), 422);
        let body: Value = resp.json().await.unwrap();
        assert_eq!(body["details"]["rule"], json!("invitation_expired"));
    }
    // A new invitation replaces the expired one.
    assert_eq!(
        invite(&app, &owner, &expired_email, "brewer")
            .await
            .status(),
        201
    );

    let resp = app
        .client
        .delete(format!(
            "{}/api/v1/members/invitations/{revoked_id}",
            app.base
        ))
        .bearer_auth(&owner)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);
    assert_eq!(preview(&app, &revoked_token).await.status(), 404);
    assert_eq!(accept(&app, &revoked_token).await.status(), 404);
}

#[tokio::test]
async fn who_may_invite_whom() {
    let app = spawn_app().await;
    let (owner, tenant_id) = app.register().await;
    let manager = member(&app, tenant_id, "manager").await;
    let brewer = member(&app, tenant_id, "brewer").await;

    assert_eq!(
        invite(&app, &owner, &new_email(), "owner").await.status(),
        201
    );
    assert_eq!(
        invite(&app, &manager, &new_email(), "viewer")
            .await
            .status(),
        201
    );
    assert_eq!(
        invite(&app, &manager, &new_email(), "manager")
            .await
            .status(),
        403
    );
    assert_eq!(
        invite(&app, &brewer, &new_email(), "viewer").await.status(),
        403
    );
    assert_eq!(
        app.get("/api/v1/members/invitations", &brewer)
            .await
            .status(),
        403
    );

    // A Manager cannot revoke an Owner's invitation for a Manager.
    let (id, _token) = invited(&app, &owner, &new_email(), "manager").await;
    let resp = app
        .client
        .delete(format!("{}/api/v1/members/invitations/{id}", app.base))
        .bearer_auth(&manager)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn existing_accounts_and_open_invitations_are_refused() {
    let app = spawn_app().await;
    let (owner, _tenant_id) = app.register().await;

    let taken = new_email();
    let resp = public_post(
        &app,
        "/api/v1/auth/register",
        json!({"email": taken, "password": PASSWORD, "display_name": "Elsewhere", "tenant_name": format!("Other {}", uniq())}),
    )
    .await;
    assert_eq!(resp.status(), 201);
    assert_eq!(invite(&app, &owner, &taken, "brewer").await.status(), 409);

    let email = new_email();
    assert_eq!(invite(&app, &owner, &email, "brewer").await.status(), 201);
    assert_eq!(
        invite(&app, &owner, &email.to_uppercase(), "viewer")
            .await
            .status(),
        409
    );
}
