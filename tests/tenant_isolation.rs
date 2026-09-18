//! Tenant isolation: tenant B must not be able to read, change or delete
//! tenant A's resources. Every probe below uses one of A's ids with B's token
//! and must get 404 (never 200/204, and never a 400/403/409/422 that would
//! reveal the id exists). Failures are collected so one run lists every leak.
//! Afterwards A must still be able to read each resource (B's DELETEs did nothing).

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
        trust_proxy_headers: false,
        redis_url: None,
        redis_key_prefix: "batchwise-test".into(),
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

/// Every feature flag, so both tenants can reach every gated route.
const ALL_FEATURES: &str = r#"{"inventory":true,"recipes":true,"batches":true,"calendar":true,"yeastkinetics":true,"library":true,"water":true,"yeast_banking":true,"fermentation":true,"tracking":true,"reporting":true,"sales":true,"duty":true,"allergens":true,"labels":true,"label_design":true,"packaging":true,"traceability":true,"equipment_maintenance":true,"procurement":true}"#;

/// One request tenant B makes against tenant A's resource.
struct Probe {
    method: reqwest::Method,
    path: String,
    body: Option<Value>,
}

fn probe(method: reqwest::Method, path: impl Into<String>, body: Option<Value>) -> Probe {
    Probe {
        method,
        path: path.into(),
        body,
    }
}

/// GET, PATCH `{}`-style body, and DELETE on one id route.
fn crud(path: &str, patch_body: Value) -> Vec<Probe> {
    vec![
        probe(reqwest::Method::GET, path, None),
        probe(reqwest::Method::PATCH, path, Some(patch_body)),
        probe(reqwest::Method::DELETE, path, None),
    ]
}

impl TestApp {
    /// Registers a fresh user + tenant with every feature enabled; returns the token.
    async fn tenant(&self) -> String {
        let body = json!({
            "email": format!("iso-{}@example.com", uniq()),
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
        assert_eq!(resp.status(), 201, "register");
        let v: Value = resp.json().await.unwrap();
        let tenant_id = Uuid::parse_str(v["tenant_id"].as_str().unwrap()).unwrap();
        let pool = sqlx::PgPool::connect(&self.db_url).await.unwrap();
        sqlx::query("UPDATE tenants SET feature_flags = feature_flags || $2::jsonb WHERE id=$1")
            .bind(tenant_id)
            .bind(ALL_FEATURES)
            .execute(&pool)
            .await
            .unwrap();
        v["access_token"].as_str().unwrap().to_string()
    }

    async fn send(
        &self,
        token: &str,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> reqwest::Response {
        let mut req = self
            .client
            .request(method, format!("{}{path}", self.base))
            .bearer_auth(token);
        if let Some(b) = body {
            req = req.json(b);
        }
        req.send().await.unwrap()
    }

    /// POSTs as `token`, asserts 2xx, returns the JSON body.
    async fn create(&self, token: &str, path: &str, body: Value) -> Value {
        let resp = self
            .send(token, reqwest::Method::POST, path, Some(&body))
            .await;
        let status = resp.status();
        let text = resp.text().await.unwrap();
        assert!(status.is_success(), "create {path}: {status} {text}");
        serde_json::from_str(&text).unwrap()
    }

    /// GETs as `token`, asserts 200, returns the JSON body.
    async fn read(&self, token: &str, path: &str) -> Value {
        let resp = self.send(token, reqwest::Method::GET, path, None).await;
        let status = resp.status();
        let text = resp.text().await.unwrap();
        assert_eq!(status, 200, "read {path}: {text}");
        serde_json::from_str(&text).unwrap()
    }

    /// Runs every probe with `intruder`'s token and panics listing all probes
    /// that did not return 404. Then checks every probed GET path still exists
    /// for `owner` (so no intruder DELETE took effect).
    async fn assert_isolated(&self, owner: &str, intruder: &str, probes: &[Probe]) {
        let mut leaks = Vec::new();
        for p in probes {
            let resp = self
                .send(intruder, p.method.clone(), &p.path, p.body.as_ref())
                .await;
            let status = resp.status();
            if status != reqwest::StatusCode::NOT_FOUND {
                let text = resp.text().await.unwrap_or_default();
                let snippet: String = text.chars().take(160).collect();
                leaks.push(format!("{} {} -> {status} {snippet}", p.method, p.path));
            }
        }
        for p in probes.iter().filter(|p| p.method == reqwest::Method::GET) {
            let resp = self.send(owner, reqwest::Method::GET, &p.path, None).await;
            // Only existence matters here: render routes may 422 an incomplete design.
            if resp.status() == reqwest::StatusCode::NOT_FOUND {
                leaks.push(format!(
                    "owner GET {} -> {} after intruder probes",
                    p.path,
                    resp.status()
                ));
            }
        }
        assert!(
            leaks.is_empty(),
            "{} isolation failures:\n{}",
            leaks.len(),
            leaks.join("\n")
        );
    }
}

fn id(v: &Value) -> String {
    v["id"].as_str().expect("id").to_string()
}

#[tokio::test]
async fn core_resources_are_isolated() {
    use reqwest::Method as M;
    let app = spawn_app().await;
    let a = app.tenant().await;
    let b = app.tenant().await;
    let mut probes: Vec<Probe> = Vec::new();

    // Inventory lot.
    let lot_body = json!({"type": "fermentable", "name": format!("Malt {}", uniq()), "amount": 10.0, "unit": "kg", "lot_number": format!("LOT-{}", uniq())});
    let lot = id(&app.create(&a, "/api/v1/inventory", lot_body.clone()).await);
    probes.extend(crud(&format!("/api/v1/inventory/{lot}"), json!({})));
    probes.push(probe(
        M::PUT,
        format!("/api/v1/inventory/{lot}"),
        Some(lot_body),
    ));
    probes.push(probe(
        M::POST,
        format!("/api/v1/inventory/{lot}/stock"),
        Some(json!({"amount": 1.0})),
    ));

    // Library: tenant-owned rows are private (system rows are shared, not tested here).
    let mut yeast_id = String::new();
    for kind in [
        "styles",
        "equipment-profiles",
        "mash-profiles",
        "yeasts",
        "fermentables",
    ] {
        let body = json!({"name": format!("Lib {}", uniq())});
        let lib = id(&app
            .create(&a, &format!("/api/v1/library/{kind}"), body.clone())
            .await);
        if kind == "yeasts" {
            yeast_id = lib.clone();
        }
        let path = format!("/api/v1/library/{kind}/{lib}");
        probes.extend(crud(&path, json!({})));
        probes.push(probe(M::PUT, path, Some(body)));
    }

    // Recipe.
    let recipe_body = json!({"name": format!("Recipe {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0});
    let recipe = id(&app.create(&a, "/api/v1/recipes", recipe_body.clone()).await);
    probes.extend(crud(&format!("/api/v1/recipes/{recipe}"), json!({})));
    // PUT validates the child arrays before loading the row, so send them.
    let mut recipe_put = recipe_body;
    for child in ["fermentables", "hops", "yeasts", "mash_steps"] {
        recipe_put[child] = json!([]);
    }
    probes.push(probe(
        M::PUT,
        format!("/api/v1/recipes/{recipe}"),
        Some(recipe_put),
    ));

    // Batch (create wraps the row in "batch").
    let batch_v = app
        .create(&a, "/api/v1/batches", json!({"recipe_id": recipe, "batch_number": format!("B-{}", uniq()), "name": "Iso Batch"}))
        .await;
    let batch = batch_v["batch"]["id"].as_str().unwrap().to_string();
    probes.extend(crud(&format!("/api/v1/batches/{batch}"), json!({})));
    probes.push(probe(
        M::POST,
        format!("/api/v1/batches/{batch}/transition"),
        Some(json!({"to_status": "brewing"})),
    ));
    probes.push(probe(
        M::PATCH,
        format!("/api/v1/batches/{batch}/ingredients"),
        Some(json!({})),
    ));

    // Fermentation reading on A's batch.
    let reading_body = json!({"gravity": 1.048, "temp_c": 19.5, "ph": 5.2});
    let reading = id(&app
        .create(
            &a,
            &format!("/api/v1/batches/{batch}/fermentation"),
            reading_body.clone(),
        )
        .await);
    probes.push(probe(
        M::GET,
        format!("/api/v1/batches/{batch}/fermentation"),
        None,
    ));
    probes.push(probe(
        M::POST,
        format!("/api/v1/batches/{batch}/fermentation"),
        Some(reading_body),
    ));
    probes.push(probe(
        M::PATCH,
        format!("/api/v1/batches/{batch}/fermentation/{reading}"),
        Some(json!({"notes": "x"})),
    ));
    probes.push(probe(
        M::DELETE,
        format!("/api/v1/batches/{batch}/fermentation/{reading}"),
        None,
    ));

    // Calendar event.
    let event = id(&app.create(&a, "/api/v1/calendar-events", json!({"event_type": "custom", "title": "Clean kegs", "start_time": "2026-07-01T09:00:00Z"})).await);
    probes.extend(crud(&format!("/api/v1/calendar-events/{event}"), json!({})));

    // Fermenter.
    let fermenter = id(&app
        .create(
            &a,
            "/api/v1/fermenters",
            json!({"name": format!("FV {}", uniq())}),
        )
        .await);
    probes.extend(crud(&format!("/api/v1/fermenters/{fermenter}"), json!({})));

    // Yeast kinetics profile referencing A's library yeast.
    let yk_body = json!({"yeast_id": yeast_id, "fermentation_temp_c": 18.0, "primary_fermentation_days": 7, "conditioning_days": 14, "lag_phase_hours": 12, "attenuation_pct": 78.0});
    let yk = id(&app
        .create(&a, "/api/v1/yeast-kinetics", yk_body.clone())
        .await);
    probes.extend(crud(&format!("/api/v1/yeast-kinetics/{yk}"), json!({})));
    // The PUT body must reference a yeast B can see, or the yeast check answers first.
    let b_yeast = id(&app
        .create(
            &b,
            "/api/v1/library/yeasts",
            json!({"name": format!("B yeast {}", uniq())}),
        )
        .await);
    let mut yk_put = yk_body;
    yk_put["yeast_id"] = json!(b_yeast);
    probes.push(probe(
        M::PUT,
        format!("/api/v1/yeast-kinetics/{yk}"),
        Some(yk_put),
    ));

    // Water profile and adjustment.
    let wp_body = json!({"name": format!("Burton {}", uniq()), "calcium_ppm": 275.0, "magnesium_ppm": 40.0, "sodium_ppm": 25.0, "sulfate_ppm": 610.0, "chloride_ppm": 35.0, "bicarbonate_ppm": 270.0});
    let wp = id(&app
        .create(&a, "/api/v1/water-profiles", wp_body.clone())
        .await);
    probes.extend(crud(&format!("/api/v1/water-profiles/{wp}"), json!({})));
    probes.push(probe(
        M::PUT,
        format!("/api/v1/water-profiles/{wp}"),
        Some(wp_body),
    ));
    let adj_body = json!({"name": format!("Adj {}", uniq()), "source_profile_id": wp, "volume_liters": 25.0, "mineral_additions": [{"type": "CaCl2", "amount": 3.0}]});
    let adj = id(&app
        .create(&a, "/api/v1/water-adjustments", adj_body.clone())
        .await);
    probes.extend(crud(&format!("/api/v1/water-adjustments/{adj}"), json!({})));
    // Like the yeast-kinetics PUT: a body naming A's profile is rejected by the
    // reference check before the adjustment lookup, so leave the optional id out.
    let mut adj_put = adj_body;
    adj_put.as_object_mut().unwrap().remove("source_profile_id");
    probes.push(probe(
        M::PUT,
        format!("/api/v1/water-adjustments/{adj}"),
        Some(adj_put),
    ));

    app.assert_isolated(&a, &b, &probes).await;
}

/// A 1x1 PNG for the brand-asset upload.
fn png_1x1() -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M8AAAMBAQDJ/pLvAAAAAElFTkSuQmCC")
        .unwrap()
}

#[tokio::test]
async fn pro_resources_are_isolated() {
    use reqwest::Method as M;
    let app = spawn_app().await;
    let a = app.tenant().await;
    let b = app.tenant().await;
    let mut probes: Vec<Probe> = Vec::new();

    // A batch with OG/FG so a sale crystallises a duty event.
    let recipe = id(&app
        .create(&a, "/api/v1/recipes", json!({"name": format!("R {}", uniq()), "type": "all_grain", "batch_size_liters": 100.0, "yeasts": [{"name": "US-05", "amount": 11.0, "unit": "g"}]}))
        .await);
    let batch_v = app
        .create(&a, "/api/v1/batches", json!({"recipe_id": recipe, "batch_number": format!("B-{}", uniq()), "name": "Pro Batch"}))
        .await;
    let batch = batch_v["batch"]["id"].as_str().unwrap().to_string();
    let resp = app
        .send(
            &a,
            M::PATCH,
            &format!("/api/v1/batches/{batch}"),
            Some(&json!({"actual_og": 1.050, "actual_fg": 1.010})),
        )
        .await;
    assert_eq!(resp.status(), 200, "set OG/FG");

    // Sales: customer, order, item, fulfilment -> duty event.
    let customer_body = json!({"name": format!("C {}", uniq()), "country": "GB"});
    let customer = id(&app
        .create(&a, "/api/v1/customers", customer_body.clone())
        .await);
    probes.extend(crud(&format!("/api/v1/customers/{customer}"), json!({})));
    probes.push(probe(
        M::PUT,
        format!("/api/v1/customers/{customer}"),
        Some(customer_body),
    ));
    let order = id(&app
        .create(&a, "/api/v1/orders", json!({"customer_id": customer}))
        .await);
    let item_body = json!({"batch_id": batch, "product_name": "Keg", "volume_liters": 50.0, "unit_price_pence": 10000, "quantity": 2});
    let item = id(&app
        .create(
            &a,
            &format!("/api/v1/orders/{order}/items"),
            item_body.clone(),
        )
        .await);
    probes.extend(crud(&format!("/api/v1/orders/{order}"), json!({})));
    for action in ["confirm", "fulfill", "invoice", "cancel"] {
        probes.push(probe(
            M::POST,
            format!("/api/v1/orders/{order}/{action}"),
            Some(json!({})),
        ));
    }
    probes.push(probe(
        M::POST,
        format!("/api/v1/orders/{order}/items"),
        Some(item_body.clone()),
    ));
    probes.push(probe(
        M::PUT,
        format!("/api/v1/orders/{order}/items/{item}"),
        Some(item_body),
    ));
    probes.push(probe(
        M::DELETE,
        format!("/api/v1/orders/{order}/items/{item}"),
        None,
    ));
    let resp = app
        .send(
            &a,
            M::POST,
            &format!("/api/v1/orders/{order}/confirm"),
            Some(&json!({})),
        )
        .await;
    assert_eq!(resp.status(), 200, "confirm order");
    let resp = app
        .send(
            &a,
            M::POST,
            &format!("/api/v1/orders/{order}/fulfill"),
            Some(&json!({})),
        )
        .await;
    assert_eq!(resp.status(), 200, "fulfil order");
    let duty_events = app.read(&a, "/api/v1/duty-events").await;
    let duty_event = duty_events["items"][0]["id"]
        .as_str()
        .expect("duty event")
        .to_string();
    probes.push(probe(
        M::GET,
        format!("/api/v1/duty-events/{duty_event}"),
        None,
    ));

    // Duty return.
    let duty_return = id(&app
        .create(
            &a,
            "/api/v1/duty-returns/compile",
            json!({"period_start": "2020-01-01", "period_end": "2035-12-31"}),
        )
        .await);
    probes.push(probe(
        M::GET,
        format!("/api/v1/duty-returns/{duty_return}"),
        None,
    ));
    probes.push(probe(
        M::PATCH,
        format!("/api/v1/duty-returns/{duty_return}"),
        Some(json!({"status": "submitted"})),
    ));

    // Packaging run, distribution movement, traceability.
    let run = id(&app
        .create(&a, "/api/v1/packaging-runs", json!({"batch_id": batch, "format": "keg", "unit_volume_ml": 50000, "quantity": 100, "lot_number": format!("PKG-{}", uniq()), "packaged_at": "2026-07-01"}))
        .await);
    probes.extend(crud(
        &format!("/api/v1/packaging-runs/{run}"),
        json!({"notes": "x"}),
    ));
    let movement = id(&app
        .create(&a, "/api/v1/distribution-movements", json!({"packaging_run_id": run, "movement_type": "sample", "quantity": 10, "to_location": "Taproom"}))
        .await);
    probes.push(probe(
        M::GET,
        format!("/api/v1/distribution-movements/{movement}"),
        None,
    ));
    probes.push(probe(
        M::POST,
        format!("/api/v1/distribution-movements/{movement}/void"),
        Some(json!({"reason": "isolation probe"})),
    ));
    probes.push(probe(
        M::GET,
        format!("/api/v1/traceability/packaging-runs/{run}"),
        None,
    ));

    // Procurement: supplier, purchase order, line.
    let supplier = id(&app
        .create(
            &a,
            "/api/v1/suppliers",
            json!({"name": format!("Supplier {}", uniq())}),
        )
        .await);
    probes.extend(crud(&format!("/api/v1/suppliers/{supplier}"), json!({})));
    let po = id(&app
        .create(
            &a,
            "/api/v1/purchase-orders",
            json!({"supplier_id": supplier}),
        )
        .await);
    let line_body = json!({"ingredient_type": "fermentable", "ingredient_name": "Maris Otter", "quantity": 25.0, "unit": "kg", "unit_cost_pence": 150});
    let line = id(&app
        .create(
            &a,
            &format!("/api/v1/purchase-orders/{po}/lines"),
            line_body.clone(),
        )
        .await);
    probes.extend(crud(&format!("/api/v1/purchase-orders/{po}"), json!({})));
    probes.push(probe(
        M::POST,
        format!("/api/v1/purchase-orders/{po}/lines"),
        Some(line_body),
    ));
    probes.push(probe(
        M::PATCH,
        format!("/api/v1/purchase-orders/{po}/lines/{line}"),
        Some(json!({"quantity": 99.0})),
    ));
    probes.push(probe(
        M::DELETE,
        format!("/api/v1/purchase-orders/{po}/lines/{line}"),
        None,
    ));
    probes.push(probe(
        M::POST,
        format!("/api/v1/purchase-orders/{po}/receive"),
        Some(json!({"lines": [{"line_id": line, "received_quantity": 1.0}]})),
    ));

    // Equipment with a schedule and an event.
    let equipment = id(&app
        .create(
            &a,
            "/api/v1/equipment",
            json!({"name": format!("Kit {}", uniq()), "equipment_type": "fermenter"}),
        )
        .await);
    probes.extend(crud(&format!("/api/v1/equipment/{equipment}"), json!({})));
    let schedule_body = json!({"task_name": "Calibrate sensor", "interval_days": 7});
    let schedule = id(&app
        .create(
            &a,
            &format!("/api/v1/equipment/{equipment}/schedules"),
            schedule_body.clone(),
        )
        .await);
    probes.push(probe(
        M::GET,
        format!("/api/v1/equipment/{equipment}/schedules"),
        None,
    ));
    probes.push(probe(
        M::POST,
        format!("/api/v1/equipment/{equipment}/schedules"),
        Some(schedule_body),
    ));
    probes.push(probe(
        M::PATCH,
        format!("/api/v1/equipment/{equipment}/schedules/{schedule}"),
        Some(json!({})),
    ));
    probes.push(probe(
        M::DELETE,
        format!("/api/v1/equipment/{equipment}/schedules/{schedule}"),
        None,
    ));
    let event_body = json!({"event_type": "service", "cost_pence": 1500});
    let event = id(&app
        .create(
            &a,
            &format!("/api/v1/equipment/{equipment}/events"),
            event_body.clone(),
        )
        .await);
    probes.push(probe(
        M::GET,
        format!("/api/v1/equipment/{equipment}/events"),
        None,
    ));
    probes.push(probe(
        M::POST,
        format!("/api/v1/equipment/{equipment}/events"),
        Some(event_body),
    ));
    probes.push(probe(
        M::DELETE,
        format!("/api/v1/equipment/{equipment}/events/{event}"),
        None,
    ));

    // Container tracking: asset, log (from a fill), QR codes.
    let asset_body = json!({"asset_number": format!("K-{}", uniq()), "container_type": "keg", "capacity_liters": 50.0});
    let asset = id(&app
        .create(&a, "/api/v1/container-assets", asset_body.clone())
        .await);
    let resp = app
        .send(
            &a,
            M::POST,
            &format!("/api/v1/container-assets/{asset}/fill"),
            Some(&json!({})),
        )
        .await;
    assert_eq!(resp.status(), 200, "fill container");
    let logs = app
        .read(&a, &format!("/api/v1/container-logs?container_id={asset}"))
        .await;
    let log = logs["items"][0]["id"]
        .as_str()
        .expect("container log")
        .to_string();
    probes.extend(crud(
        &format!("/api/v1/container-assets/{asset}"),
        json!({}),
    ));
    probes.push(probe(
        M::PUT,
        format!("/api/v1/container-assets/{asset}"),
        Some(asset_body),
    ));
    probes.push(probe(
        M::POST,
        format!("/api/v1/container-assets/{asset}/fill"),
        Some(json!({})),
    ));
    probes.push(probe(
        M::POST,
        format!("/api/v1/container-assets/{asset}/deliver"),
        Some(json!({"customer_name": "Pub"})),
    ));
    probes.push(probe(
        M::POST,
        format!("/api/v1/container-assets/{asset}/return"),
        Some(json!({})),
    ));
    probes.push(probe(
        M::POST,
        format!("/api/v1/container-assets/{asset}/status"),
        Some(json!({"to_status": "empty"})),
    ));
    probes.push(probe(M::GET, format!("/api/v1/container-logs/{log}"), None));
    probes.push(probe(M::GET, format!("/api/v1/qr-codes/{asset}/a"), None));
    probes.push(probe(M::GET, format!("/api/v1/qr-codes/{asset}/b"), None));

    // Reporting: cost rate, batch cost, cost report.
    let rate_body = json!({"rate_type": "energy", "rate_name": "Electricity", "unit": "pence_per_kwh", "rate_value": 30.0, "effective_from": "2026-01-01"});
    let rate = id(&app
        .create(&a, "/api/v1/cost-rates", rate_body.clone())
        .await);
    probes.extend(crud(&format!("/api/v1/cost-rates/{rate}"), json!({})));
    probes.push(probe(
        M::PUT,
        format!("/api/v1/cost-rates/{rate}"),
        Some(rate_body),
    ));
    let resp = app
        .send(
            &a,
            M::POST,
            "/api/v1/batch-costs/compute",
            Some(&json!({"batch_id": batch})),
        )
        .await;
    assert!(
        resp.status().is_success(),
        "compute batch cost: {}",
        resp.status()
    );
    probes.push(probe(M::GET, format!("/api/v1/batch-costs/{batch}"), None));
    probes.push(probe(
        M::POST,
        "/api/v1/batch-costs/compute",
        Some(json!({"batch_id": batch})),
    ));
    let report = id(&app
        .create(
            &a,
            "/api/v1/cost-reports/generate",
            json!({"report_type": "batch", "batch_id": batch}),
        )
        .await);
    probes.push(probe(
        M::GET,
        format!("/api/v1/cost-reports/{report}"),
        None,
    ));
    probes.push(probe(
        M::DELETE,
        format!("/api/v1/cost-reports/{report}"),
        None,
    ));
    probes.push(probe(
        M::POST,
        "/api/v1/cost-reports/generate",
        Some(json!({"report_type": "batch", "batch_id": batch})),
    ));

    // Label record.
    let label = id(&app
        .create(
            &a,
            "/api/v1/label-records",
            json!({"batch_id": batch, "net_volume_ml": 500}),
        )
        .await);
    probes.extend(crud(&format!("/api/v1/label-records/{label}"), json!({})));

    // Label design: brand profile, brand asset, design.
    let profile = id(&app
        .create(
            &a,
            "/api/v1/brand-profiles",
            json!({"name": format!("House {}", uniq())}),
        )
        .await);
    probes.extend(crud(
        &format!("/api/v1/brand-profiles/{profile}"),
        json!({}),
    ));
    let part = reqwest::multipart::Part::bytes(png_1x1())
        .file_name("logo.png")
        .mime_str("image/png")
        .unwrap();
    let resp = app
        .client
        .post(format!("{}/api/v1/brand-assets", app.base))
        .bearer_auth(&a)
        .multipart(reqwest::multipart::Form::new().part("file", part))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "upload brand asset");
    let asset_v: Value = resp.json().await.unwrap();
    let brand_asset = id(&asset_v);
    probes.push(probe(
        M::GET,
        format!("/api/v1/brand-assets/{brand_asset}"),
        None,
    ));
    probes.push(probe(
        M::DELETE,
        format!("/api/v1/brand-assets/{brand_asset}"),
        None,
    ));
    let design = id(&app
        .create(&a, "/api/v1/label-designs", json!({"kind": "bottle", "name": "My Bottle", "batch_id": batch, "size_key": "bottle_front_90x120", "template_key": "compliance_standard"}))
        .await);
    probes.extend(crud(&format!("/api/v1/label-designs/{design}"), json!({})));
    probes.push(probe(
        M::GET,
        format!("/api/v1/label-designs/{design}/render"),
        None,
    ));
    probes.push(probe(
        M::GET,
        format!("/api/v1/label-designs/{design}/render.pdf"),
        None,
    ));

    // Yeast bank entry and propagation.
    let entry = id(&app
        .create(&a, "/api/v1/yeast-bank", json!({"name": format!("US-05 {}", uniq()), "harvested_at": "2026-06-01T00:00:00Z", "viability_percent": 90.0, "quantity_ml": 500.0}))
        .await);
    probes.extend(crud(&format!("/api/v1/yeast-bank/{entry}"), json!({})));
    probes.push(probe(
        M::POST,
        format!("/api/v1/yeast-bank/{entry}/harvest"),
        Some(json!({"viability_percent": 95.0})),
    ));
    let prop = id(&app
        .create(
            &a,
            &format!("/api/v1/yeast-bank/{entry}/propagations"),
            json!({"volume_ml": 1000.0}),
        )
        .await);
    probes.push(probe(
        M::GET,
        format!("/api/v1/yeast-bank/{entry}/propagations"),
        None,
    ));
    probes.push(probe(
        M::POST,
        format!("/api/v1/yeast-bank/{entry}/propagations"),
        Some(json!({"volume_ml": 1000.0})),
    ));
    probes.push(probe(
        M::PATCH,
        format!("/api/v1/yeast-bank/{entry}/propagations/{prop}"),
        Some(json!({"volume_ml": 1500.0})),
    ));
    probes.push(probe(
        M::DELETE,
        format!("/api/v1/yeast-bank/{entry}/propagations/{prop}"),
        None,
    ));

    // Compliance audit event written by A's activity above.
    let audit = app.read(&a, "/api/v1/compliance-audit").await;
    let audit_id = audit["items"][0]["id"]
        .as_str()
        .expect("audit event")
        .to_string();
    probes.push(probe(
        M::GET,
        format!("/api/v1/compliance-audit/{audit_id}"),
        None,
    ));

    app.assert_isolated(&a, &b, &probes).await;
}

/// One request tenant B makes with one of tenant A's ids in its body.
struct RefCase {
    label: &'static str,
    method: reqwest::Method,
    path: String,
    body: Value,
    foreign_id: String,
}

fn ref_case(
    label: &'static str,
    method: reqwest::Method,
    path: impl Into<String>,
    body: Value,
    foreign_id: &str,
) -> RefCase {
    RefCase {
        label,
        method,
        path: path.into(),
        body,
        foreign_id: foreign_id.to_string(),
    }
}

impl TestApp {
    /// Sends each case twice as `intruder`: once with the other tenant's id and
    /// once with a random id. The foreign id must be rejected, with the same
    /// status as the random id, so a response never reveals that the id exists.
    async fn assert_foreign_refs_rejected(&self, intruder: &str, cases: &[RefCase]) {
        let mut leaks = Vec::new();
        for c in cases {
            let foreign = self
                .send(intruder, c.method.clone(), &c.path, Some(&c.body))
                .await;
            let foreign_status = foreign.status();
            let foreign_text = foreign.text().await.unwrap_or_default();

            let random_body: Value = serde_json::from_str(
                &c.body
                    .to_string()
                    .replace(&c.foreign_id, &Uuid::new_v4().to_string()),
            )
            .unwrap();
            let random = self
                .send(intruder, c.method.clone(), &c.path, Some(&random_body))
                .await;
            let random_status = random.status();

            if foreign_status.is_success() || foreign_status != random_status {
                let snippet: String = foreign_text.chars().take(160).collect();
                leaks.push(format!(
                    "{}: {} {} -> foreign {foreign_status}, random {random_status} {snippet}",
                    c.label, c.method, c.path
                ));
            }
        }
        assert!(
            leaks.is_empty(),
            "{} foreign-reference failures:\n{}",
            leaks.len(),
            leaks.join("\n")
        );
    }
}

#[tokio::test]
async fn foreign_references_are_rejected() {
    use reqwest::Method as M;
    let app = spawn_app().await;
    let a = app.tenant().await;
    let b = app.tenant().await;

    // ---- Tenant A's resources (the foreign ids) ----
    let a_style = id(&app
        .create(
            &a,
            "/api/v1/library/styles",
            json!({"name": format!("A style {}", uniq())}),
        )
        .await);
    let a_equip_profile = id(&app
        .create(
            &a,
            "/api/v1/library/equipment-profiles",
            json!({"name": format!("A kit {}", uniq())}),
        )
        .await);
    let a_mash = id(&app
        .create(
            &a,
            "/api/v1/library/mash-profiles",
            json!({"name": format!("A mash {}", uniq())}),
        )
        .await);
    let a_yeast = id(&app
        .create(
            &a,
            "/api/v1/library/yeasts",
            json!({"name": format!("A yeast {}", uniq())}),
        )
        .await);
    let a_lot = id(&app
        .create(&a, "/api/v1/inventory", json!({"type": "fermentable", "name": format!("A malt {}", uniq()), "amount": 10.0, "unit": "kg", "lot_number": format!("LOT-{}", uniq())}))
        .await);
    let a_recipe = id(&app.create(&a, "/api/v1/recipes", json!({"name": format!("A recipe {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0})).await);
    let a_batch_v = app
        .create(&a, "/api/v1/batches", json!({"recipe_id": a_recipe, "batch_number": format!("A-{}", uniq()), "name": "A batch"}))
        .await;
    let a_batch = a_batch_v["batch"]["id"].as_str().unwrap().to_string();
    let a_fermenter = id(&app
        .create(
            &a,
            "/api/v1/fermenters",
            json!({"name": format!("A FV {}", uniq())}),
        )
        .await);
    let a_run = id(&app
        .create(&a, "/api/v1/packaging-runs", json!({"batch_id": a_batch, "format": "keg", "unit_volume_ml": 50000, "quantity": 100, "lot_number": format!("PKG-{}", uniq()), "packaged_at": "2026-07-01"}))
        .await);
    let a_customer = id(&app
        .create(
            &a,
            "/api/v1/customers",
            json!({"name": format!("A cust {}", uniq()), "country": "GB"}),
        )
        .await);
    let a_order = id(&app
        .create(&a, "/api/v1/orders", json!({"customer_id": a_customer}))
        .await);
    let a_supplier = id(&app
        .create(
            &a,
            "/api/v1/suppliers",
            json!({"name": format!("A supplier {}", uniq())}),
        )
        .await);
    let a_po = id(&app
        .create(
            &a,
            "/api/v1/purchase-orders",
            json!({"supplier_id": a_supplier}),
        )
        .await);
    let a_line = id(&app
        .create(&a, &format!("/api/v1/purchase-orders/{a_po}/lines"), json!({"ingredient_type": "fermentable", "ingredient_name": "Maris Otter", "quantity": 25.0, "unit": "kg", "unit_cost_pence": 150}))
        .await);
    let a_equipment = id(&app
        .create(
            &a,
            "/api/v1/equipment",
            json!({"name": format!("A kit {}", uniq()), "equipment_type": "fermenter"}),
        )
        .await);
    let a_schedule = id(&app
        .create(
            &a,
            &format!("/api/v1/equipment/{a_equipment}/schedules"),
            json!({"task_name": "Calibrate", "interval_days": 7}),
        )
        .await);
    let a_water = id(&app
        .create(&a, "/api/v1/water-profiles", json!({"name": format!("A water {}", uniq()), "calcium_ppm": 275.0, "magnesium_ppm": 40.0, "sodium_ppm": 25.0, "sulfate_ppm": 610.0, "chloride_ppm": 35.0, "bicarbonate_ppm": 270.0}))
        .await);
    let a_profile = id(&app
        .create(
            &a,
            "/api/v1/brand-profiles",
            json!({"name": format!("A house {}", uniq())}),
        )
        .await);
    let part = reqwest::multipart::Part::bytes(png_1x1())
        .file_name("logo.png")
        .mime_str("image/png")
        .unwrap();
    let resp = app
        .client
        .post(format!("{}/api/v1/brand-assets", app.base))
        .bearer_auth(&a)
        .multipart(reqwest::multipart::Form::new().part("file", part))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "upload brand asset");
    let a_asset_v: Value = resp.json().await.unwrap();
    let a_asset = id(&a_asset_v);

    // ---- Tenant B's own resources (targets for PATCH / sub-actions) ----
    let b_yeast = id(&app
        .create(
            &b,
            "/api/v1/library/yeasts",
            json!({"name": format!("B yeast {}", uniq())}),
        )
        .await);
    let b_recipe = id(&app.create(&b, "/api/v1/recipes", json!({"name": format!("B recipe {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0})).await);
    let b_batch_v = app
        .create(&b, "/api/v1/batches", json!({"recipe_id": b_recipe, "batch_number": format!("B-{}", uniq()), "name": "B batch"}))
        .await;
    let b_batch = b_batch_v["batch"]["id"].as_str().unwrap().to_string();
    let b_event = id(&app.create(&b, "/api/v1/calendar-events", json!({"event_type": "custom", "title": "B event", "start_time": "2026-07-01T09:00:00Z"})).await);
    let b_run = id(&app
        .create(&b, "/api/v1/packaging-runs", json!({"batch_id": b_batch, "format": "keg", "unit_volume_ml": 50000, "quantity": 100, "lot_number": format!("PKG-{}", uniq()), "packaged_at": "2026-07-01"}))
        .await);
    let b_customer = id(&app
        .create(
            &b,
            "/api/v1/customers",
            json!({"name": format!("B cust {}", uniq()), "country": "GB"}),
        )
        .await);
    let b_order = id(&app
        .create(&b, "/api/v1/orders", json!({"customer_id": b_customer}))
        .await);
    let b_supplier = id(&app
        .create(
            &b,
            "/api/v1/suppliers",
            json!({"name": format!("B supplier {}", uniq())}),
        )
        .await);
    let b_po = id(&app
        .create(
            &b,
            "/api/v1/purchase-orders",
            json!({"supplier_id": b_supplier}),
        )
        .await);
    let b_equipment = id(&app
        .create(
            &b,
            "/api/v1/equipment",
            json!({"name": format!("B kit {}", uniq()), "equipment_type": "fermenter"}),
        )
        .await);
    let b_asset = id(&app
        .create(&b, "/api/v1/container-assets", json!({"asset_number": format!("K-{}", uniq()), "container_type": "keg", "capacity_liters": 50.0}))
        .await);
    let b_profile = id(&app
        .create(
            &b,
            "/api/v1/brand-profiles",
            json!({"name": format!("B house {}", uniq())}),
        )
        .await);
    let design_body = || json!({"kind": "bottle", "name": format!("B bottle {}", uniq()), "batch_id": b_batch, "size_key": "bottle_front_90x120", "template_key": "compliance_standard"});
    let b_design = id(&app.create(&b, "/api/v1/label-designs", design_body()).await);
    let adjustment_body = || json!({"name": format!("B adj {}", uniq()), "volume_liters": 25.0, "mineral_additions": [{"type": "CaCl2", "amount": 3.0}]});
    let b_adjustment = id(&app
        .create(&b, "/api/v1/water-adjustments", adjustment_body())
        .await);
    let entry_body = || json!({"name": format!("B US-05 {}", uniq()), "harvested_at": "2026-06-01T00:00:00Z", "viability_percent": 90.0, "quantity_ml": 500.0});
    let b_entry = id(&app.create(&b, "/api/v1/yeast-bank", entry_body()).await);
    let b_prop = id(&app
        .create(
            &b,
            &format!("/api/v1/yeast-bank/{b_entry}/propagations"),
            json!({"volume_ml": 1000.0}),
        )
        .await);
    let yk_body = json!({"yeast_id": b_yeast, "fermentation_temp_c": 18.0, "primary_fermentation_days": 7, "conditioning_days": 14, "lag_phase_hours": 12, "attenuation_pct": 78.0});
    let b_yk = id(&app
        .create(&b, "/api/v1/yeast-kinetics", yk_body.clone())
        .await);

    let with = |base: &Value, field: &str, value: &str| {
        let mut v = base.clone();
        v[field] = json!(value);
        v
    };
    // Bases are closures so every case gets a unique name/number: otherwise a
    // leaked create makes later cases fail on a duplicate and hides their leaks.
    let recipe_base = || json!({"name": format!("B r {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0});
    let batch_base = || json!({"recipe_id": b_recipe, "batch_number": format!("B2-{}", uniq()), "name": "B batch 2"});
    let event_base =
        json!({"event_type": "custom", "title": "B event 2", "start_time": "2026-07-01T09:00:00Z"});
    let run_base = || json!({"format": "keg", "unit_volume_ml": 50000, "quantity": 10, "lot_number": format!("PKG-{}", uniq()), "packaged_at": "2026-07-01"});
    let item_base = json!({"product_name": "Keg", "volume_liters": 50.0, "unit_price_pence": 10000, "quantity": 1});

    let cases = vec![
        // batches
        ref_case(
            "batch.recipe_id",
            M::POST,
            "/api/v1/batches",
            json!({"recipe_id": a_recipe, "batch_number": format!("X-{}", uniq()), "name": "x"}),
            &a_recipe,
        ),
        ref_case(
            "batch.fermenter_id (create)",
            M::POST,
            "/api/v1/batches",
            with(&batch_base(), "fermenter_id", &a_fermenter),
            &a_fermenter,
        ),
        ref_case(
            "batch.fermenter_id (patch)",
            M::PATCH,
            format!("/api/v1/batches/{b_batch}"),
            json!({"fermenter_id": a_fermenter}),
            &a_fermenter,
        ),
        // calendar
        ref_case(
            "calendar.batch_id (create)",
            M::POST,
            "/api/v1/calendar-events",
            with(&event_base, "batch_id", &a_batch),
            &a_batch,
        ),
        ref_case(
            "calendar.batch_id (patch)",
            M::PATCH,
            format!("/api/v1/calendar-events/{b_event}"),
            json!({"batch_id": a_batch}),
            &a_batch,
        ),
        // equipment
        ref_case(
            "equipment event.schedule_id",
            M::POST,
            format!("/api/v1/equipment/{b_equipment}/events"),
            json!({"event_type": "service", "schedule_id": a_schedule}),
            &a_schedule,
        ),
        // inventory
        ref_case(
            "inventory deduct.preferred_lot_id",
            M::POST,
            "/api/v1/inventory/deduct",
            json!({"type": "fermentable", "name": "Anything", "amount": 1.0, "unit": "kg", "preferred_lot_id": a_lot}),
            &a_lot,
        ),
        // label design
        ref_case(
            "brand profile.logo_asset_id (create)",
            M::POST,
            "/api/v1/brand-profiles",
            json!({"name": format!("B p {}", uniq()), "logo_asset_id": a_asset}),
            &a_asset,
        ),
        ref_case(
            "brand profile.logo_asset_id (patch)",
            M::PATCH,
            format!("/api/v1/brand-profiles/{b_profile}"),
            json!({"logo_asset_id": a_asset}),
            &a_asset,
        ),
        ref_case(
            "label design.batch_id",
            M::POST,
            "/api/v1/label-designs",
            with(&design_body(), "batch_id", &a_batch),
            &a_batch,
        ),
        ref_case(
            "label design.recipe_id",
            M::POST,
            "/api/v1/label-designs",
            json!({"kind": "cask_lens", "name": "B lens", "recipe_id": a_recipe, "size_key": "lens_round_100", "template_key": "compliance_standard"}),
            &a_recipe,
        ),
        ref_case(
            "label design.brand_profile_id (create)",
            M::POST,
            "/api/v1/label-designs",
            with(&design_body(), "brand_profile_id", &a_profile),
            &a_profile,
        ),
        ref_case(
            "label design.brand_profile_id (patch)",
            M::PATCH,
            format!("/api/v1/label-designs/{b_design}"),
            json!({"brand_profile_id": a_profile}),
            &a_profile,
        ),
        // labels
        ref_case(
            "label record.batch_id",
            M::POST,
            "/api/v1/label-records",
            json!({"batch_id": a_batch, "net_volume_ml": 500}),
            &a_batch,
        ),
        // packaging
        ref_case(
            "packaging run.batch_id",
            M::POST,
            "/api/v1/packaging-runs",
            with(&run_base(), "batch_id", &a_batch),
            &a_batch,
        ),
        ref_case(
            "movement.packaging_run_id",
            M::POST,
            "/api/v1/distribution-movements",
            json!({"packaging_run_id": a_run, "movement_type": "sample", "quantity": 1, "to_location": "x"}),
            &a_run,
        ),
        ref_case(
            "movement.order_id",
            M::POST,
            "/api/v1/distribution-movements",
            json!({"packaging_run_id": b_run, "movement_type": "sale", "quantity": 1, "order_id": a_order}),
            &a_order,
        ),
        // procurement
        ref_case(
            "purchase order.supplier_id",
            M::POST,
            "/api/v1/purchase-orders",
            json!({"supplier_id": a_supplier}),
            &a_supplier,
        ),
        ref_case(
            "receive.line_id",
            M::POST,
            format!("/api/v1/purchase-orders/{b_po}/receive"),
            json!({"lines": [{"line_id": a_line, "received_quantity": 1.0}]}),
            &a_line,
        ),
        // recipes
        ref_case(
            "recipe.style_id (create)",
            M::POST,
            "/api/v1/recipes",
            with(&recipe_base(), "style_id", &a_style),
            &a_style,
        ),
        ref_case(
            "recipe.equipment_profile_id",
            M::POST,
            "/api/v1/recipes",
            with(&recipe_base(), "equipment_profile_id", &a_equip_profile),
            &a_equip_profile,
        ),
        ref_case(
            "recipe.mash_profile_id",
            M::POST,
            "/api/v1/recipes",
            with(&recipe_base(), "mash_profile_id", &a_mash),
            &a_mash,
        ),
        ref_case(
            "recipe.style_id (patch)",
            M::PATCH,
            format!("/api/v1/recipes/{b_recipe}"),
            json!({"style_id": a_style}),
            &a_style,
        ),
        ref_case(
            "recipe fermentable.inventory_lot_id",
            M::POST,
            "/api/v1/recipes",
            {
                let mut v = recipe_base();
                v["fermentables"] = json!([{"step_order": 1, "name": "Malt", "amount": 5.0, "unit": "kg", "inventory_lot_id": a_lot}]);
                v
            },
            &a_lot,
        ),
        ref_case(
            "recipe yeast.yeast_id",
            M::POST,
            "/api/v1/recipes",
            {
                let mut v = recipe_base();
                v["yeasts"] =
                    json!([{"name": "Yeast", "amount": 11.0, "unit": "g", "yeast_id": a_yeast}]);
                v
            },
            &a_yeast,
        ),
        // reporting
        ref_case(
            "batch cost.batch_id",
            M::POST,
            "/api/v1/batch-costs/compute",
            json!({"batch_id": a_batch}),
            &a_batch,
        ),
        ref_case(
            "cost report.batch_id",
            M::POST,
            "/api/v1/cost-reports/generate",
            json!({"report_type": "batch", "batch_id": a_batch}),
            &a_batch,
        ),
        ref_case(
            "cost report.recipe_id",
            M::POST,
            "/api/v1/cost-reports/generate",
            json!({"report_type": "recipe", "recipe_id": a_recipe}),
            &a_recipe,
        ),
        // sales
        ref_case(
            "order.customer_id",
            M::POST,
            "/api/v1/orders",
            json!({"customer_id": a_customer}),
            &a_customer,
        ),
        ref_case(
            "order item.batch_id",
            M::POST,
            format!("/api/v1/orders/{b_order}/items"),
            with(&item_base, "batch_id", &a_batch),
            &a_batch,
        ),
        // tracking
        ref_case(
            "container fill.batch_id",
            M::POST,
            format!("/api/v1/container-assets/{b_asset}/fill"),
            json!({"batch_id": a_batch}),
            &a_batch,
        ),
        // water
        ref_case(
            "water adj.source_profile_id (create)",
            M::POST,
            "/api/v1/water-adjustments",
            with(&adjustment_body(), "source_profile_id", &a_water),
            &a_water,
        ),
        ref_case(
            "water adj.target_profile_id",
            M::POST,
            "/api/v1/water-adjustments",
            with(&adjustment_body(), "target_profile_id", &a_water),
            &a_water,
        ),
        ref_case(
            "water adj.batch_id",
            M::POST,
            "/api/v1/water-adjustments",
            with(&adjustment_body(), "batch_id", &a_batch),
            &a_batch,
        ),
        ref_case(
            "water adj.recipe_id",
            M::POST,
            "/api/v1/water-adjustments",
            with(&adjustment_body(), "recipe_id", &a_recipe),
            &a_recipe,
        ),
        ref_case(
            "water adj.source_profile_id (patch)",
            M::PATCH,
            format!("/api/v1/water-adjustments/{b_adjustment}"),
            json!({"source_profile_id": a_water}),
            &a_water,
        ),
        ref_case(
            "water calculate.source_profile_id",
            M::POST,
            "/api/v1/water-adjustments/calculate",
            json!({"source_profile_id": a_water, "volume_liters": 20.0, "mineral_additions": [{"type": "CaSO4", "amount": 5.0}]}),
            &a_water,
        ),
        // yeast bank
        ref_case(
            "yeast bank.library_yeast_id (create)",
            M::POST,
            "/api/v1/yeast-bank",
            with(&entry_body(), "library_yeast_id", &a_yeast),
            &a_yeast,
        ),
        ref_case(
            "yeast bank.library_yeast_id (patch)",
            M::PATCH,
            format!("/api/v1/yeast-bank/{b_entry}"),
            json!({"library_yeast_id": a_yeast}),
            &a_yeast,
        ),
        ref_case(
            "propagation.batch_id (create)",
            M::POST,
            format!("/api/v1/yeast-bank/{b_entry}/propagations"),
            json!({"volume_ml": 1000.0, "batch_id": a_batch}),
            &a_batch,
        ),
        ref_case(
            "propagation.batch_id (patch)",
            M::PATCH,
            format!("/api/v1/yeast-bank/{b_entry}/propagations/{b_prop}"),
            json!({"batch_id": a_batch}),
            &a_batch,
        ),
        // yeast kinetics
        ref_case(
            "yeast kinetics.yeast_id (create)",
            M::POST,
            "/api/v1/yeast-kinetics",
            with(&yk_body, "yeast_id", &a_yeast),
            &a_yeast,
        ),
        ref_case(
            "yeast kinetics.yeast_id (patch)",
            M::PATCH,
            format!("/api/v1/yeast-kinetics/{b_yk}"),
            json!({"yeast_id": a_yeast}),
            &a_yeast,
        ),
    ];

    app.assert_foreign_refs_rejected(&b, &cases).await;
}
