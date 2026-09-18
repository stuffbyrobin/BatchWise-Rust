//! Rate limiting across instances, against a real Redis.
//!
//! Runs only when `TEST_REDIS_URL` is set (CI, and locally with a Redis on
//! hand); without it the tests report that they were skipped and pass, so the
//! suite still runs on a machine without Redis.

use std::net::SocketAddr;

use batchwise::platform::config::Config;
use batchwise::platform::database;
use batchwise::state::AppState;
use serde_json::{json, Value};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

fn uniq() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn test_config(
    database_url: String,
    redis_url: Option<String>,
    prefix: &str,
    login_limit: u32,
) -> Config {
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
        rate_limit_login_per_minute: login_limit,
        rate_limit_refresh_per_minute: 1000,
        rate_limit_default_per_minute: 1000,
        trust_proxy_headers: false,
        redis_url,
        redis_key_prefix: prefix.into(),
        migrations_disabled: false,
        log_level: "info".into(),
    }
}

/// A Postgres to share between the instances: the one from `TEST_DATABASE_URL`,
/// or a container.
async fn database() -> (String, Option<ContainerAsync<Postgres>>) {
    match std::env::var("TEST_DATABASE_URL") {
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
    }
}

/// Starts one instance and returns its base URL.
async fn spawn(db: &str, redis_url: Option<String>, prefix: &str, login_limit: u32) -> String {
    let pool = database::connect(db).await.expect("connect");
    database::migrate(&pool).await.expect("migrate");
    let state = AppState::new(
        pool,
        test_config(db.to_string(), redis_url, prefix, login_limit),
    );
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
    format!("http://{addr}")
}

async fn register(client: &reqwest::Client, base: &str, email: &str, password: &str) {
    let resp = client
        .post(format!("{base}/api/v1/auth/register"))
        .json(&json!({
            "email": email,
            "password": password,
            "display_name": "Tester",
            "tenant_name": format!("Brewery {}", uniq()),
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "register");
}

async fn login(
    client: &reqwest::Client,
    base: &str,
    email: &str,
    password: &str,
) -> reqwest::Response {
    client
        .post(format!("{base}/api/v1/auth/login"))
        .json(&json!({"email": email, "password": password}))
        .send()
        .await
        .unwrap()
}

/// `TEST_REDIS_URL`, or `None` with a note that the test was skipped.
fn redis_url(test: &str) -> Option<String> {
    match std::env::var("TEST_REDIS_URL") {
        Ok(url) if !url.trim().is_empty() => Some(url),
        _ => {
            eprintln!("skipping {test}: TEST_REDIS_URL is not set");
            None
        }
    }
}

#[tokio::test]
async fn instances_sharing_redis_share_one_limit() {
    let Some(redis) = redis_url("instances_sharing_redis_share_one_limit") else {
        return;
    };
    let (db, _node) = database().await;
    let prefix = format!("rl-shared-{}", uniq());
    let a = spawn(&db, Some(redis.clone()), &prefix, 3).await;
    let b = spawn(&db, Some(redis), &prefix, 3).await;

    let client = reqwest::Client::new();
    let email = format!("shared-{}@example.com", uniq());
    let password = "Sup3rSecret!pw";
    register(&client, &a, &email, password).await;

    // Three logins is the limit, wherever they land.
    assert_eq!(login(&client, &a, &email, password).await.status(), 200);
    assert_eq!(login(&client, &b, &email, password).await.status(), 200);
    assert_eq!(login(&client, &a, &email, password).await.status(), 200);

    let resp = login(&client, &b, &email, password).await;
    assert_eq!(
        resp.status(),
        429,
        "the fourth login is over the shared limit"
    );
    let retry: u64 = resp
        .headers()
        .get("retry-after")
        .expect("retry-after")
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((1..=60).contains(&retry), "retry-after {retry}");
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["code"], json!("rate_limited"));

    // Another route keeps its own window.
    let other = format!("other-{}@example.com", uniq());
    register(&client, &b, &other, password).await;
}

#[tokio::test]
async fn instances_without_redis_limit_on_their_own() {
    let (db, _node) = database().await;
    let a = spawn(&db, None, "unused", 3).await;
    let b = spawn(&db, None, "unused", 3).await;

    let client = reqwest::Client::new();
    let email = format!("local-{}@example.com", uniq());
    let password = "Sup3rSecret!pw";
    register(&client, &a, &email, password).await;

    for _ in 0..3 {
        assert_eq!(login(&client, &a, &email, password).await.status(), 200);
    }
    assert_eq!(login(&client, &a, &email, password).await.status(), 429);
    // The second instance has its own window.
    assert_eq!(login(&client, &b, &email, password).await.status(), 200);
}

#[tokio::test]
async fn an_unreachable_redis_falls_back_to_the_local_window() {
    let (db, _node) = database().await;
    // Nothing listens on port 1.
    let base = spawn(&db, Some("redis://127.0.0.1:1".into()), "rl-down", 3).await;

    let client = reqwest::Client::new();
    let email = format!("down-{}@example.com", uniq());
    let password = "Sup3rSecret!pw";
    register(&client, &base, &email, password).await;

    for _ in 0..3 {
        assert_eq!(login(&client, &base, &email, password).await.status(), 200);
    }
    assert_eq!(
        login(&client, &base, &email, password).await.status(),
        429,
        "the in-memory window still limits while Redis is down"
    );
}

#[tokio::test]
async fn failed_logins_are_counted_across_instances() {
    let Some(redis) = redis_url("failed_logins_are_counted_across_instances") else {
        return;
    };
    let (db, _node) = database().await;
    let prefix = format!("rl-failed-{}", uniq());
    let a = spawn(&db, Some(redis.clone()), &prefix, 1000).await;
    let b = spawn(&db, Some(redis), &prefix, 1000).await;

    let client = reqwest::Client::new();
    let email = format!("failed-{}@example.com", uniq());
    let password = "Sup3rSecret!pw";
    register(&client, &a, &email, password).await;

    // Ten wrong passwords, split between the instances.
    for i in 0..10 {
        let base = if i % 2 == 0 { &a } else { &b };
        assert_eq!(
            login(&client, base, &email, "Wrong-Password-123!")
                .await
                .status(),
            401
        );
    }
    // The account is locked on both, even with the right password.
    assert_eq!(login(&client, &b, &email, password).await.status(), 429);
    assert_eq!(login(&client, &a, &email, password).await.status(), 429);
}
