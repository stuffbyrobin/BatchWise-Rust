//! Serves the hand-authored OpenAPI spec and a Swagger UI page.
//!
//! Port of the Go `GET /api/v1/openapi.yaml` and `GET /api/v1/docs` endpoints.
//! Both are public (no auth) so tooling and docs can be fetched directly.

use axum::http::header;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;

/// The OpenAPI 3.1 document, embedded at compile time.
const OPENAPI_YAML: &str = include_str!("../openapi.yaml");

const SWAGGER_UI: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>Batchwise API — Swagger UI</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    window.ui = SwaggerUIBundle({ url: '/api/v1/openapi.yaml', dom_id: '#swagger-ui' });
  </script>
</body>
</html>"#;

/// Public OpenAPI routes (`/openapi.yaml`, `/docs`).
pub fn routes() -> Router {
    Router::new()
        .route("/openapi.yaml", get(spec))
        .route("/docs", get(docs))
}

async fn spec() -> Response {
    (
        [(header::CONTENT_TYPE, "application/yaml; charset=utf-8")],
        OPENAPI_YAML,
    )
        .into_response()
}

async fn docs() -> Html<&'static str> {
    Html(SWAGGER_UI)
}
