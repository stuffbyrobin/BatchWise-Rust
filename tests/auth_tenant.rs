//! Integration tests for the auth and tenant modules against a real Postgres
//! (via `testcontainers`, the Rust analogue of testcontainers-go).
//!
//! Covers the required Phase 1 scenarios: registration, login (incl. global
//! email uniqueness), refresh-token rotation + reuse, `/me`, tenant read/update,
//! owner-only updates, and password policy.

use std::net::SocketAddr;

use batchwise::platform::config::Config;
use batchwise::platform::database;
use batchwise::state::AppState;
use serde_json::{json, Value};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

/// A running app instance bound to an ephemeral port, plus the DB container
/// (kept alive for the lifetime of the test).
struct TestApp {
    base: String,
    client: reqwest::Client,
    _node: Option<ContainerAsync<Postgres>>,
}

/// Unique suffix so tests are idempotent across re-runs and safe to run in
/// parallel against a shared database.
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
    // Escape hatch for environments where the Docker registry is unreachable:
    // point TEST_DATABASE_URL at an existing Postgres. Otherwise spin a
    // throwaway container (used in CI).
    let (url, node) = match std::env::var("TEST_DATABASE_URL") {
        Ok(url) => (url, None),
        Err(_) => {
            let node = Postgres::default()
                .with_tag("16-alpine")
                .start()
                .await
                .expect("start postgres");
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
    async fn post(&self, path: &str, body: Value) -> reqwest::Response {
        self.client
            .post(format!("{}{path}", self.base))
            .json(&body)
            .send()
            .await
            .unwrap()
    }

    async fn register(&self, email: &str, tenant: Option<&str>) -> Value {
        let mut body = json!({
            "email": email,
            "password": "Sup3rSecret!pw",
            "display_name": "Tester",
        });
        if let Some(t) = tenant {
            body["tenant_name"] = json!(t);
        }
        let resp = self.post("/api/v1/auth/register", body).await;
        assert_eq!(resp.status(), 201, "register should return 201");
        assert!(resp.headers().contains_key("location"));
        resp.json().await.unwrap()
    }

    async fn login(&self, email: &str, password: &str) -> Value {
        let resp = self
            .post(
                "/api/v1/auth/login",
                json!({"email": email, "password": password}),
            )
            .await;
        assert_eq!(resp.status(), 200, "login");
        resp.json().await.unwrap()
    }

    /// Status of `GET /auth/me` with `access_token`.
    async fn me_status(&self, access_token: &Value) -> u16 {
        self.client
            .get(format!("{}/api/v1/auth/me", self.base))
            .bearer_auth(access_token.as_str().unwrap())
            .send()
            .await
            .unwrap()
            .status()
            .as_u16()
    }
}

#[tokio::test]
async fn register_login_refresh_and_me_flow() {
    let app = spawn_app().await;

    // Register (creates a tenant; user is owner).
    let email = format!("owner-{}@example.com", uniq());
    let tenant = format!("Hoppy Brewing {}", uniq());
    let reg = app.register(&email, Some(&tenant)).await;
    assert_eq!(reg["role"], json!("owner"));
    assert_eq!(reg["token_type"], json!("Bearer"));
    let access = reg["access_token"].as_str().unwrap().to_string();
    let refresh = reg["refresh_token"].as_str().unwrap().to_string();

    // /me with the bearer token.
    let resp = app
        .client
        .get(format!("{}/api/v1/auth/me", app.base))
        .bearer_auth(&access)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let me: Value = resp.json().await.unwrap();
    assert_eq!(me["email"], json!(email));
    assert_eq!(me["tenant_name"], json!(tenant));
    assert_eq!(me["tier"], json!("home"));

    // Login as that user.
    let resp = app
        .post(
            "/api/v1/auth/login",
            json!({"email": email, "password": "Sup3rSecret!pw"}),
        )
        .await;
    assert_eq!(resp.status(), 200);

    // Wrong password → 401.
    let resp = app
        .post(
            "/api/v1/auth/login",
            json!({"email": email, "password": "wrong-password"}),
        )
        .await;
    assert_eq!(resp.status(), 401);

    // Refresh rotates the token.
    let resp = app
        .post("/api/v1/auth/refresh", json!({"refresh_token": refresh}))
        .await;
    assert_eq!(resp.status(), 200);

    // Reusing the now-rotated token → 401.
    let resp = app
        .post("/api/v1/auth/refresh", json!({"refresh_token": refresh}))
        .await;
    assert_eq!(resp.status(), 401);

    // The replay ends every session, so the access token is refused too.
    assert_eq!(app.me_status(&json!(access)).await, 401);
}

#[tokio::test]
async fn email_is_globally_unique_across_tenants() {
    let app = spawn_app().await;
    let email = format!("dup-{}@example.com", uniq());
    app.register(&email, Some(&format!("Tenant One {}", uniq())))
        .await;

    // Same email, different tenant → 409.
    let resp = app
        .post(
            "/api/v1/auth/register",
            json!({
                "email": email,
                "password": "Sup3rSecret!pw",
                "display_name": "Tester",
                "tenant_name": format!("Tenant Two {}", uniq()),
            }),
        )
        .await;
    assert_eq!(resp.status(), 409);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["code"], json!("conflict"));
    assert!(!body["request_id"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn weak_password_is_rejected() {
    let app = spawn_app().await;
    let resp = app
        .post(
            "/api/v1/auth/register",
            json!({
                "email": format!("weak-{}@example.com", uniq()),
                "password": "short",
                "display_name": "Tester",
                "tenant_name": format!("Weak Co {}", uniq()),
            }),
        )
        .await;
    assert_eq!(resp.status(), 400);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["code"], json!("validation_error"));
}

#[tokio::test]
async fn tenant_read_and_owner_only_update() {
    let app = spawn_app().await;

    // Owner of a tenant.
    let cask = format!("Cask Co {}", uniq());
    let owner = app
        .register(&format!("boss-{}@example.com", uniq()), Some(&cask))
        .await;
    let owner_token = owner["access_token"].as_str().unwrap().to_string();

    // GET /tenants/current.
    let resp = app
        .client
        .get(format!("{}/api/v1/tenants/current", app.base))
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let tn: Value = resp.json().await.unwrap();
    assert_eq!(tn["tenant_name"], json!(cask));

    // Owner can PATCH.
    let renamed = format!("Cask & Keg Co {}", uniq());
    let resp = app
        .client
        .patch(format!("{}/api/v1/tenants/current", app.base))
        .bearer_auth(&owner_token)
        .json(&json!({"tenant_name": renamed}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let tn: Value = resp.json().await.unwrap();
    assert_eq!(tn["tenant_name"], json!(renamed));

    // A bootstrap (non-owner, system-tenant) user cannot PATCH.
    let member = app
        .register(&format!("member-{}@example.com", uniq()), None)
        .await;
    assert_eq!(member["role"], json!("manager"));
    let member_token = member["access_token"].as_str().unwrap().to_string();
    let resp = app
        .client
        .patch(format!("{}/api/v1/tenants/current", app.base))
        .bearer_auth(&member_token)
        .json(&json!({"tenant_name": "Hijack Co"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn protected_routes_require_auth() {
    let app = spawn_app().await;
    let resp = app
        .client
        .get(format!("{}/api/v1/auth/me", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    let resp = app
        .client
        .get(format!("{}/api/v1/tenants/current", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn refresh_replay_revokes_token_family() {
    let app = spawn_app().await;
    let email = format!("revoke-{}+{}", uniq(), "@example.com");
    let tenant = format!("Revoke Tenant {}", uniq());
    let reg = app.register(&email, Some(&tenant)).await;
    let r1 = reg["refresh_token"].as_str().unwrap().to_string();

    let resp = app
        .post("/api/v1/auth/refresh", json!({"refresh_token": r1}))
        .await;
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let r2 = body["refresh_token"].as_str().unwrap().to_string();
    assert_eq!(app.me_status(&body["access_token"]).await, 200);

    let resp = app
        .post("/api/v1/auth/refresh", json!({"refresh_token": r1}))
        .await;
    assert_eq!(resp.status(), 401);

    let resp = app
        .post("/api/v1/auth/refresh", json!({"refresh_token": r2}))
        .await;
    assert_eq!(resp.status(), 401);

    // Access tokens from before the replay are revoked as well.
    assert_eq!(app.me_status(&body["access_token"]).await, 401);
    assert_eq!(app.me_status(&reg["access_token"]).await, 401);
}

#[tokio::test]
async fn concurrent_refresh_yields_exactly_one_success() {
    let app = spawn_app().await;
    let email = format!("concurrent-{}+{}", uniq(), "@example.com");
    let reg = app.register(&email, None).await;
    let r1 = reg["refresh_token"].as_str().unwrap().to_string();

    let f1 = app.post("/api/v1/auth/refresh", json!({"refresh_token": r1.clone()}));
    let f2 = app.post("/api/v1/auth/refresh", json!({"refresh_token": r1.clone()}));
    let f3 = app.post("/api/v1/auth/refresh", json!({"refresh_token": r1.clone()}));
    let f4 = app.post("/api/v1/auth/refresh", json!({"refresh_token": r1.clone()}));
    let f5 = app.post("/api/v1/auth/refresh", json!({"refresh_token": r1.clone()}));

    let responses = tokio::join!(f1, f2, f3, f4, f5);
    let statuses = [
        responses.0.status(),
        responses.1.status(),
        responses.2.status(),
        responses.3.status(),
        responses.4.status(),
    ];
    let success_count = statuses.iter().filter(|&&s| s == 200).count();
    assert_eq!(success_count, 1);
    for s in statuses {
        assert!(s == 200 || s == 401);
    }
}

#[tokio::test]
async fn repeated_failed_logins_lock_the_account() {
    let app = spawn_app().await;
    let email = format!("locked-{}+{}", uniq(), "@example.com");
    let password = "Sup3rSecret!pw";
    app.register(&email, None).await;

    for i in 0..10 {
        let resp = app
            .post(
                "/api/v1/auth/login",
                json!({"email": email, "password": "Wrong-Password-123!"}),
            )
            .await;
        assert_eq!(resp.status(), 401, "attempt {i}");
    }

    let resp = app
        .post(
            "/api/v1/auth/login",
            json!({"email": email, "password": password}),
        )
        .await;
    assert_eq!(resp.status(), 429);
}

#[tokio::test]
async fn logout_revokes_the_access_token_sent_with_it() {
    let app = spawn_app().await;
    let email = format!("logout-{}@example.com", uniq());
    let first = app.register(&email, None).await;
    let second = app.login(&email, "Sup3rSecret!pw").await;
    assert_eq!(app.me_status(&first["access_token"]).await, 200);

    let resp = app
        .client
        .post(format!("{}/api/v1/auth/logout", app.base))
        .bearer_auth(first["access_token"].as_str().unwrap())
        .json(&json!({"refresh_token": first["refresh_token"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // That session is over, and the other one is untouched.
    assert_eq!(app.me_status(&first["access_token"]).await, 401);
    assert_eq!(app.me_status(&second["access_token"]).await, 200);

    // A garbage bearer token does not break logout.
    let resp = app
        .client
        .post(format!("{}/api/v1/auth/logout", app.base))
        .bearer_auth("not.a.jwt")
        .json(&json!({"refresh_token": "unknown"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Without an access token, logout ends the refresh token only.
    let resp = app
        .post(
            "/api/v1/auth/logout",
            json!({"refresh_token": second["refresh_token"]}),
        )
        .await;
    assert_eq!(resp.status(), 204);
    assert_eq!(app.me_status(&second["access_token"]).await, 200);

    // Refreshing with a logged-out token is indistinguishable from a stolen one
    // being replayed, so it ends every session the user has.
    let resp = app
        .post(
            "/api/v1/auth/refresh",
            json!({"refresh_token": first["refresh_token"]}),
        )
        .await;
    assert_eq!(resp.status(), 401);
    assert_eq!(app.me_status(&second["access_token"]).await, 401);
}

#[tokio::test]
async fn password_change_ends_every_session() {
    let app = spawn_app().await;
    let email = format!("pwchange-{}@example.com", uniq());
    let first = app.register(&email, None).await;
    let second = app.login(&email, "Sup3rSecret!pw").await;

    let resp = app
        .client
        .patch(format!("{}/api/v1/auth/me", app.base))
        .bearer_auth(first["access_token"].as_str().unwrap())
        .json(&json!({"current_password": "Sup3rSecret!pw", "new_password": "N3w-Sup3rSecret!pw"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    assert_eq!(app.me_status(&first["access_token"]).await, 401);
    assert_eq!(app.me_status(&second["access_token"]).await, 401);
    let resp = app
        .post(
            "/api/v1/auth/refresh",
            json!({"refresh_token": second["refresh_token"]}),
        )
        .await;
    assert_eq!(resp.status(), 401);

    let fresh = app.login(&email, "N3w-Sup3rSecret!pw").await;
    assert_eq!(app.me_status(&fresh["access_token"]).await, 200);
}
