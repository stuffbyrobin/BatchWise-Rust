//! The [`ApiError`] type and its JSON rendering.
//!
//! Port of the Go `internal/platform/errors` package. The wire shape is
//! identical: `{ "code", "message", "details"?, "request_id" }`.

use std::collections::BTreeMap;

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

/// A structured API error carrying an HTTP status, a stable machine code, a
/// human message, and optional structured details.
#[derive(Debug, Clone)]
pub struct ApiError {
    code: &'static str,
    message: String,
    status: StatusCode,
    details: Option<BTreeMap<String, Value>>,
    /// Populated for `rate_limited`; emitted as the `Retry-After` header.
    retry_after_seconds: Option<u64>,
    /// Captured internal cause; logged but never serialised.
    source: Option<String>,
}

impl ApiError {
    /// 400 — a specific field failed validation.
    pub fn validation(field: &str, reason: &str) -> Self {
        let mut d = BTreeMap::new();
        d.insert("field".into(), json!(field));
        d.insert("reason".into(), json!(reason));
        Self::base(
            "validation_error",
            "Validation failed.",
            StatusCode::BAD_REQUEST,
        )
        .with_details(d)
    }

    /// 404 — the named resource does not exist (or is cross-tenant).
    pub fn not_found(resource: &str) -> Self {
        Self::base(
            "not_found",
            &format!("{resource} not found"),
            StatusCode::NOT_FOUND,
        )
    }

    /// 409 — a uniqueness or state conflict.
    pub fn conflict(resource: &str, reason: &str) -> Self {
        Self::base(
            "conflict",
            &format!("{resource}: {reason}"),
            StatusCode::CONFLICT,
        )
    }

    /// 401 — authentication missing or invalid.
    pub fn unauthorized(message: &str) -> Self {
        Self::base("unauthorized", message, StatusCode::UNAUTHORIZED)
    }

    /// 403 — authenticated but not permitted.
    pub fn forbidden(reason: &str) -> Self {
        Self::base("forbidden", reason, StatusCode::FORBIDDEN)
    }

    /// 422 — a business rule was violated. `rule` is merged into details.
    pub fn business_rule(rule: &str, message: &str, details: BTreeMap<String, Value>) -> Self {
        let mut merged = BTreeMap::new();
        merged.insert("rule".into(), json!(rule));
        merged.extend(details);
        Self::base(
            "business_rule_violation",
            message,
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .with_details(merged)
    }

    /// 415 — request body media type is not `application/json`.
    pub fn unsupported_media_type() -> Self {
        Self::base(
            "unsupported_media_type",
            "unsupported media type",
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        )
    }

    /// 429 — rate limit exceeded; sets a `Retry-After` header.
    pub fn rate_limited(retry_after_seconds: u64) -> Self {
        let mut e = Self::base(
            "rate_limited",
            "rate limit exceeded",
            StatusCode::TOO_MANY_REQUESTS,
        );
        e.retry_after_seconds = Some(retry_after_seconds);
        e
    }

    /// 500 — an unexpected internal error. The cause is logged, never returned.
    pub fn internal(err: impl std::fmt::Display) -> Self {
        let mut e = Self::base(
            "internal_error",
            "internal server error",
            StatusCode::INTERNAL_SERVER_ERROR,
        );
        e.source = Some(err.to_string());
        e
    }

    /// HTTP status this error renders as.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Stable machine-readable code.
    pub fn code(&self) -> &'static str {
        self.code
    }

    fn base(code: &'static str, message: &str, status: StatusCode) -> Self {
        Self {
            code,
            message: message.to_string(),
            status,
            details: None,
            retry_after_seconds: None,
            source: None,
        }
    }

    fn with_details(mut self, details: BTreeMap<String, Value>) -> Self {
        self.details = Some(details);
        self
    }

    /// Renders the error to an axum [`Response`], stamping the request id into
    /// the body. Internal (5xx) errors are logged here with their cause.
    pub fn into_response_with_request_id(self, request_id: &str) -> Response {
        if self.status.is_server_error() {
            tracing::error!(
                code = self.code,
                request_id,
                error = self.source.as_deref().unwrap_or(""),
                "internal server error"
            );
        }

        let mut body = serde_json::Map::new();
        body.insert("code".into(), json!(self.code));
        body.insert("message".into(), json!(self.message));
        if let Some(details) = &self.details {
            body.insert("details".into(), json!(details));
        }
        body.insert("request_id".into(), json!(request_id));

        let mut resp = (self.status, Json(Value::Object(body))).into_response();
        if let Some(secs) = self.retry_after_seconds {
            if let Ok(v) = secs.to_string().parse() {
                resp.headers_mut().insert(header::RETRY_AFTER, v);
            }
        }
        resp
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

/// Fallback rendering when no request id is available (e.g. errors raised
/// before the request-id middleware). Prefer [`ApiError::into_response_with_request_id`].
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        self.into_response_with_request_id("")
    }
}

/// Map a `sqlx` error to an [`ApiError`]: missing rows become 404, everything
/// else becomes an internal error.
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => ApiError::not_found("resource"),
            other => ApiError::internal(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_match_constructors() {
        assert_eq!(
            ApiError::validation("a", "b").status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(ApiError::not_found("x").status(), StatusCode::NOT_FOUND);
        assert_eq!(ApiError::conflict("x", "y").status(), StatusCode::CONFLICT);
        assert_eq!(
            ApiError::unauthorized("x").status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(ApiError::forbidden("x").status(), StatusCode::FORBIDDEN);
        assert_eq!(
            ApiError::business_rule("r", "m", Default::default()).status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            ApiError::rate_limited(30).status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            ApiError::internal("boom").status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn row_not_found_maps_to_404() {
        let e: ApiError = sqlx::Error::RowNotFound.into();
        assert_eq!(e.status(), StatusCode::NOT_FOUND);
    }
}
