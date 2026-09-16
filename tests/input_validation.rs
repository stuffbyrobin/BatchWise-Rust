//! Input validation: every request that CREATE rejects must also be rejected
//! when the same field arrives in a PATCH, request arrays are capped, imported
//! recipes are validated like typed-in ones, and uploads must really be images.
//! Each case must produce a 400 `validation_error` naming the offending field
//! (`*` accepts any field). Failures are collected so one run lists every gap.

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
}

fn id(v: &Value) -> String {
    v["id"].as_str().expect("id").to_string()
}

fn long(n: usize) -> String {
    "x".repeat(n)
}

/// One request that must be rejected with 400 `validation_error` on `field`.
struct Invalid {
    method: reqwest::Method,
    path: String,
    body: Value,
    field: &'static str,
}

fn invalid(
    method: reqwest::Method,
    path: impl Into<String>,
    body: Value,
    field: &'static str,
) -> Invalid {
    Invalid {
        method,
        path: path.into(),
        body,
        field,
    }
}

impl TestApp {
    /// Sends every case and panics listing each one that was not a 400
    /// `validation_error` on the expected field.
    async fn assert_all_invalid(&self, token: &str, cases: &[Invalid]) {
        let mut gaps = Vec::new();
        for c in cases {
            let resp = self
                .send(token, c.method.clone(), &c.path, Some(&c.body))
                .await;
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
            let field = v["details"]["field"].as_str().unwrap_or("");
            let field_ok = c.field == "*" || field == c.field;
            if status != 400 || v["code"] != "validation_error" || !field_ok {
                let snippet: String = text.chars().take(160).collect();
                gaps.push(format!(
                    "{} {} (expect field {}) -> {status} {snippet}",
                    c.method, c.path, c.field
                ));
            }
        }
        assert!(
            gaps.is_empty(),
            "{} validation gaps:\n{}",
            gaps.len(),
            gaps.join("\n")
        );
    }

    /// A recipe and a batch for tests that need them; returns (recipe_id, batch_id).
    async fn recipe_and_batch(&self, token: &str) -> (String, String) {
        let recipe = id(&self
            .create(token, "/api/v1/recipes", json!({"name": format!("R {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0}))
            .await);
        let batch = self
            .create(token, "/api/v1/batches", json!({"recipe_id": recipe, "batch_number": format!("B-{}", uniq()), "name": "V batch"}))
            .await;
        (recipe, batch["batch"]["id"].as_str().unwrap().to_string())
    }
}

#[tokio::test]
async fn patch_rejects_what_create_rejects() {
    use reqwest::Method as M;
    let app = spawn_app().await;
    let t = app.tenant().await;
    let mut cases = Vec::new();

    let lot = id(&app
        .create(&t, "/api/v1/inventory", json!({"type": "hop", "name": format!("Hop {}", uniq()), "amount": 1.0, "unit": "kg", "lot_number": format!("LOT-{}", uniq())}))
        .await);
    let p = format!("/api/v1/inventory/{lot}");
    for (body, field) in [
        (json!({"type": "bogus"}), "type"),
        (json!({"name": ""}), "name"),
        (json!({"amount": -1.0}), "amount"),
        (json!({"unit": "parsec"}), "unit"),
        (json!({"lot_number": "bad lot!"}), "lot_number"),
        (json!({"cost_pence": -1}), "cost_pence"),
        (json!({"cost_currency": "usd"}), "cost_currency"),
        (json!({"supplier": long(256)}), "supplier"),
        (json!({"origin": long(101)}), "origin"),
        (json!({"color_ebc": -1.0}), "color_ebc"),
        (json!({"alpha_acid_pct": 101.0}), "alpha_acid_pct"),
        (json!({"attenuation_pct": 101.0}), "attenuation_pct"),
        (json!({"notes": long(1001)}), "notes"),
    ] {
        cases.push(invalid(M::PATCH, &p, body, field));
    }

    for kind in [
        "styles",
        "equipment-profiles",
        "mash-profiles",
        "yeasts",
        "fermentables",
    ] {
        let lib = id(&app
            .create(
                &t,
                &format!("/api/v1/library/{kind}"),
                json!({"name": format!("Lib {}", uniq())}),
            )
            .await);
        cases.push(invalid(
            M::PATCH,
            format!("/api/v1/library/{kind}/{lib}"),
            json!({"name": ""}),
            "name",
        ));
    }

    let (recipe, batch) = app.recipe_and_batch(&t).await;
    let p = format!("/api/v1/recipes/{recipe}");
    for (body, field) in [
        (json!({"name": ""}), "name"),
        (json!({"type": "bogus"}), "type"),
        (json!({"batch_size_liters": 0.0}), "batch_size_liters"),
        (json!({"boil_size_liters": 0.0}), "boil_size_liters"),
        (json!({"boil_time_minutes": -1}), "boil_time_minutes"),
        (json!({"efficiency_pct": 101.0}), "efficiency_pct"),
        (json!({"tasting_aroma": long(1001)}), "tasting_aroma"),
        (json!({"notes": long(5001)}), "notes"),
    ] {
        cases.push(invalid(M::PATCH, &p, body, field));
    }

    let design = id(&app
        .create(&t, "/api/v1/label-designs", json!({"kind": "bottle", "name": format!("D {}", uniq()), "batch_id": batch, "size_key": "bottle_front_90x120", "template_key": "compliance_standard"}))
        .await);
    let p = format!("/api/v1/label-designs/{design}");
    cases.push(invalid(M::PATCH, &p, json!({"size_key": ""}), "size_key"));
    cases.push(invalid(
        M::PATCH,
        &p,
        json!({"template_key": ""}),
        "template_key",
    ));

    let supplier = id(&app
        .create(
            &t,
            "/api/v1/suppliers",
            json!({"name": format!("S {}", uniq())}),
        )
        .await);
    let p = format!("/api/v1/suppliers/{supplier}");
    for (body, field) in [
        (json!({"email": "not-an-email"}), "email"),
        (json!({"contact_name": long(201)}), "contact_name"),
        (json!({"phone": long(51)}), "phone"),
        (json!({"website": long(501)}), "website"),
        (json!({"notes": long(2001)}), "notes"),
    ] {
        cases.push(invalid(M::PATCH, &p, body, field));
    }
    let po = id(&app
        .create(
            &t,
            "/api/v1/purchase-orders",
            json!({"supplier_id": supplier}),
        )
        .await);
    cases.push(invalid(
        M::PATCH,
        format!("/api/v1/purchase-orders/{po}"),
        json!({"notes": long(2001)}),
        "notes",
    ));

    let rate = id(&app
        .create(&t, "/api/v1/reporting/cost-rates", json!({"rate_type": "energy", "rate_name": "Electricity", "unit": "pence_per_kwh", "rate_value": 30.0, "effective_from": "2026-01-01"}))
        .await);
    let p = format!("/api/v1/reporting/cost-rates/{rate}");
    for (body, field) in [
        (json!({"rate_name": ""}), "rate_name"),
        (json!({"unit": ""}), "unit"),
        (json!({"rate_value": 0.0}), "rate_value"),
        (json!({"effective_from": ""}), "effective_from"),
    ] {
        cases.push(invalid(M::PATCH, &p, body, field));
    }

    let customer = id(&app
        .create(
            &t,
            "/api/v1/customers",
            json!({"name": format!("C {}", uniq()), "country": "GB"}),
        )
        .await);
    let p = format!("/api/v1/customers/{customer}");
    cases.push(invalid(M::PATCH, &p, json!({"name": ""}), "name"));
    cases.push(invalid(M::PATCH, &p, json!({"country": "GBR"}), "country"));

    let asset = id(&app
        .create(&t, "/api/v1/container-assets", json!({"asset_number": format!("K-{}", uniq()), "container_type": "keg", "capacity_liters": 50.0}))
        .await);
    let p = format!("/api/v1/container-assets/{asset}");
    cases.push(invalid(
        M::PATCH,
        &p,
        json!({"asset_number": ""}),
        "asset_number",
    ));
    cases.push(invalid(
        M::PATCH,
        &p,
        json!({"container_type": "bogus"}),
        "container_type",
    ));
    cases.push(invalid(
        M::PATCH,
        &p,
        json!({"capacity_liters": 0.0}),
        "capacity_liters",
    ));

    let profile = id(&app
        .create(&t, "/api/v1/water-profiles", json!({"name": format!("W {}", uniq()), "calcium_ppm": 50.0, "magnesium_ppm": 5.0, "sodium_ppm": 10.0, "sulfate_ppm": 50.0, "chloride_ppm": 50.0, "bicarbonate_ppm": 50.0}))
        .await);
    let p = format!("/api/v1/water-profiles/{profile}");
    cases.push(invalid(M::PATCH, &p, json!({"name": ""}), "name"));
    cases.push(invalid(
        M::PATCH,
        &p,
        json!({"calcium_ppm": -1.0}),
        "calcium_ppm",
    ));
    let adjustment = id(&app
        .create(&t, "/api/v1/water-adjustments", json!({"name": format!("A {}", uniq()), "volume_liters": 20.0, "mineral_additions": [{"type": "CaCl2", "amount": 1.0}]}))
        .await);
    let p = format!("/api/v1/water-adjustments/{adjustment}");
    cases.push(invalid(M::PATCH, &p, json!({"name": ""}), "name"));
    cases.push(invalid(
        M::PATCH,
        &p,
        json!({"volume_liters": 0.0}),
        "volume_liters",
    ));

    let yeast = id(&app
        .create(
            &t,
            "/api/v1/library/yeasts",
            json!({"name": format!("Y {}", uniq())}),
        )
        .await);
    let kinetics = id(&app
        .create(&t, "/api/v1/yeast-kinetics", json!({"yeast_id": yeast, "fermentation_temp_c": 18.0, "primary_fermentation_days": 7, "conditioning_days": 14, "lag_phase_hours": 12, "attenuation_pct": 78.0}))
        .await);
    let p = format!("/api/v1/yeast-kinetics/{kinetics}");
    for (body, field) in [
        (json!({"fermentation_temp_c": 41.0}), "fermentation_temp_c"),
        (
            json!({"primary_fermentation_days": 0}),
            "primary_fermentation_days",
        ),
        (json!({"conditioning_days": 366}), "conditioning_days"),
        (json!({"lag_phase_hours": 169}), "lag_phase_hours"),
        (json!({"attenuation_pct": 101.0}), "attenuation_pct"),
        (json!({"notes": long(501)}), "notes"),
    ] {
        cases.push(invalid(M::PATCH, &p, body, field));
    }

    app.assert_all_invalid(&t, &cases).await;
}

#[tokio::test]
async fn sales_and_tenant_rules() {
    use reqwest::Method as M;
    let app = spawn_app().await;
    let t = app.tenant().await;
    let customer = id(&app
        .create(
            &t,
            "/api/v1/customers",
            json!({"name": format!("C {}", uniq()), "country": "GB"}),
        )
        .await);
    let order = id(&app
        .create(&t, "/api/v1/orders", json!({"customer_id": customer}))
        .await);

    let cases = vec![
        invalid(
            M::POST,
            format!("/api/v1/orders/{order}/items"),
            json!({"product_name": "Keg", "volume_liters": 50.0, "unit_price_pence": 100, "quantity": 0}),
            "quantity",
        ),
        invalid(
            M::PATCH,
            "/api/v1/tenants/current",
            json!({"ibu_method": "bogus"}),
            "ibu_method",
        ),
        invalid(
            M::PATCH,
            "/api/v1/tenants/current",
            json!({"next_batch_number": 0}),
            "next_batch_number",
        ),
        invalid(
            M::PATCH,
            "/api/v1/tenants/current",
            json!({"next_order_number": 0}),
            "next_order_number",
        ),
    ];
    app.assert_all_invalid(&t, &cases).await;
}

#[tokio::test]
async fn request_arrays_are_capped() {
    use reqwest::Method as M;
    let app = spawn_app().await;
    let t = app.tenant().await;
    let (recipe, batch) = app.recipe_and_batch(&t).await;

    let fermentables: Vec<Value> = (1..=101)
        .map(|i| json!({"step_order": i, "name": "Malt", "amount": 1.0, "unit": "kg"}))
        .collect();
    // Batch ingredients reuse the recipe row shape, which carries ids.
    let batch_fermentables: Vec<Value> = (1..=101)
        .map(|i| json!({"id": Uuid::nil(), "recipe_id": recipe, "step_order": i, "name": "Malt", "amount": 1.0, "unit": "kg"}))
        .collect();
    let hops: Vec<Value> = (1..=101)
        .map(|i| json!({"step_order": i, "name": "Hop", "amount": 1.0, "unit": "g", "alpha_acid_pct": 5.0, "boil_time_minutes": 60.0}))
        .collect();
    let minerals: Vec<Value> = (0..51)
        .map(|_| json!({"type": "CaCl2", "amount": 1.0}))
        .collect();
    let receive: Vec<Value> = (0..501)
        .map(|_| json!({"line_id": Uuid::new_v4(), "received_quantity": 1.0}))
        .collect();
    let allergens: Vec<String> = (0..51).map(|i| format!("a{i}")).collect();
    let steps: Vec<Value> = (1..=51)
        .map(|i| json!({"step_order": i, "step_type": "infusion", "target_temp_c": 65.0, "hold_minutes": 60}))
        .collect();

    let supplier = id(&app
        .create(
            &t,
            "/api/v1/suppliers",
            json!({"name": format!("S {}", uniq())}),
        )
        .await);
    let po = id(&app
        .create(
            &t,
            "/api/v1/purchase-orders",
            json!({"supplier_id": supplier}),
        )
        .await);
    let label = id(&app
        .create(
            &t,
            "/api/v1/label-records",
            json!({"batch_id": batch, "net_volume_ml": 500}),
        )
        .await);

    let cases = vec![
        invalid(
            M::PATCH,
            format!("/api/v1/label-records/{label}"),
            json!({"allergens": allergens.clone()}),
            "allergens",
        ),
        invalid(
            M::POST,
            "/api/v1/recipes",
            json!({"name": format!("Big {}", uniq()), "type": "all_grain", "batch_size_liters": 20.0, "fermentables": fermentables}),
            "fermentables",
        ),
        invalid(
            M::PATCH,
            format!("/api/v1/recipes/{recipe}"),
            json!({"hops": hops}),
            "hops",
        ),
        invalid(
            M::PATCH,
            format!("/api/v1/batches/{batch}/ingredients"),
            json!({"fermentables": batch_fermentables}),
            "fermentables",
        ),
        invalid(
            M::POST,
            "/api/v1/water-adjustments",
            json!({"name": format!("A {}", uniq()), "volume_liters": 20.0, "mineral_additions": minerals}),
            "mineral_additions",
        ),
        invalid(
            M::POST,
            "/api/v1/water-adjustments/calculate",
            json!({"source_profile": {"calcium_ppm": 0.0, "magnesium_ppm": 0.0, "sodium_ppm": 0.0, "sulfate_ppm": 0.0, "chloride_ppm": 0.0, "bicarbonate_ppm": 0.0}, "volume_liters": 20.0, "mineral_additions": minerals}),
            "mineral_additions",
        ),
        invalid(
            M::POST,
            format!("/api/v1/purchase-orders/{po}/receive"),
            json!({"lines": receive}),
            "lines",
        ),
        invalid(
            M::POST,
            "/api/v1/inventory",
            json!({"type": "hop", "name": format!("Hop {}", uniq()), "amount": 1.0, "unit": "kg", "lot_number": format!("LOT-{}", uniq()), "allergens": allergens}),
            "allergens",
        ),
        invalid(
            M::POST,
            "/api/v1/library/mash-profiles",
            json!({"name": format!("M {}", uniq()), "mash_steps": steps}),
            "mash_steps",
        ),
    ];
    app.assert_all_invalid(&t, &cases).await;
}

#[tokio::test]
async fn imported_recipes_are_validated() {
    use reqwest::Method as M;
    let app = spawn_app().await;
    let t = app.tenant().await;
    let xml = include_str!("../testdata/sample.xml");
    assert!(xml.contains("<AMOUNT>4.5</AMOUNT>") && xml.contains("<NAME>Sample IPA</NAME>"));
    let encode = |s: String| base64::engine::general_purpose::STANDARD.encode(s);

    let nan_amount = xml.replacen("<AMOUNT>4.5</AMOUNT>", "<AMOUNT>NaN</AMOUNT>", 1);
    let long_name = xml.replacen(
        "<NAME>Sample IPA</NAME>",
        &format!("<NAME>{}</NAME>", long(10_000)),
        1,
    );
    let cases = vec![
        invalid(
            M::POST,
            "/api/v1/recipes/import",
            json!({"format": "beerxml", "data": encode(nan_amount)}),
            "*",
        ),
        invalid(
            M::POST,
            "/api/v1/recipes/import",
            json!({"format": "beerxml", "data": encode(long_name)}),
            "name",
        ),
    ];
    app.assert_all_invalid(&t, &cases).await;
}

#[tokio::test]
async fn batch_cost_inputs_are_bounded() {
    use reqwest::Method as M;
    let app = spawn_app().await;
    let t = app.tenant().await;
    let (_recipe, batch) = app.recipe_and_batch(&t).await;
    let path = "/api/v1/reporting/batch-costs/compute";
    let cases = vec![
        invalid(
            M::POST,
            path,
            json!({"batch_id": batch, "energy_kwh": -1.0}),
            "energy_kwh",
        ),
        invalid(
            M::POST,
            path,
            json!({"batch_id": batch, "labor_hours": 1.0e12}),
            "labor_hours",
        ),
        invalid(
            M::POST,
            path,
            json!({"batch_id": batch, "water_liters": -1.0}),
            "water_liters",
        ),
        invalid(
            M::POST,
            path,
            json!({"batch_id": batch, "overhead_pence": -1}),
            "overhead_pence",
        ),
    ];
    app.assert_all_invalid(&t, &cases).await;
}

/// Minimal CRC-32 (IEEE) so the test can forge a well-formed PNG header.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A PNG signature plus an IHDR chunk declaring `width` x `height` pixels, with
/// no image data: enough for a decoder to read the dimensions.
fn png_header(width: u32, height: u32) -> Vec<u8> {
    let mut ihdr = b"IHDR".to_vec();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend_from_slice(&13u32.to_be_bytes());
    png.extend_from_slice(&ihdr);
    png.extend_from_slice(&crc32(&ihdr).to_be_bytes());
    png
}

#[tokio::test]
async fn brand_assets_must_be_real_images() {
    let app = spawn_app().await;
    let t = app.tenant().await;
    let mut gaps = Vec::new();
    for (label, bytes) in [
        ("text labelled as png", b"hello, not an image".to_vec()),
        ("100000x100000 png", png_header(100_000, 100_000)),
    ] {
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name("logo.png")
            .mime_str("image/png")
            .unwrap();
        let resp = app
            .client
            .post(format!("{}/api/v1/brand-assets", app.base))
            .bearer_auth(&t)
            .multipart(reqwest::multipart::Form::new().part("file", part))
            .send()
            .await
            .unwrap();
        let status = resp.status();
        let v: Value = resp.json().await.unwrap_or(Value::Null);
        if status != 400 || v["details"]["field"] != "file" {
            gaps.push(format!("{label} -> {status} {v}"));
        }
    }
    assert!(gaps.is_empty(), "upload gaps:\n{}", gaps.join("\n"));
}
