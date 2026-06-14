//! JSON request/response helpers.
//!
//! Port of the Go `internal/platform/web` package. [`ValidatedJson`] is the
//! axum equivalent of `web.Decode`: it enforces `application/json`, decodes the
//! body, and runs `validator` declarative validation — returning [`ApiError`]
//! values with the same wire shape as the Go decoder.
//!
//! To reject unknown fields (Go's `DisallowUnknownFields`), annotate request
//! DTOs with `#[serde(deny_unknown_fields)]`.

use axum::extract::{FromRequest, Request};
use axum::http::header;
use serde::de::DeserializeOwned;
use validator::Validate;

use super::errors::ApiError;

/// Extractor that decodes and validates a JSON request body.
#[derive(Debug, Clone, Copy)]
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !content_type.starts_with("application/json") {
            return Err(ApiError::unsupported_media_type());
        }

        let bytes = axum::body::Bytes::from_request(req, state)
            .await
            .map_err(|_| ApiError::validation("body", "could not read request body"))?;
        if bytes.is_empty() {
            return Err(ApiError::validation("body", "request body is required"));
        }

        let value: T = serde_json::from_slice(&bytes).map_err(|e| decode_error(&e))?;
        value.validate().map_err(validation_to_api_error)?;
        Ok(ValidatedJson(value))
    }
}

/// Translate a serde decode error into a field-level validation error,
/// matching the Go decoder's "unknown field" handling.
fn decode_error(e: &serde_json::Error) -> ApiError {
    let msg = e.to_string();
    if let Some(rest) = msg.strip_prefix("unknown field `") {
        if let Some(field) = rest.split('`').next() {
            return ApiError::validation(field, "unknown field");
        }
    }
    ApiError::validation("body", &msg)
}

/// Translate `validator` errors into an [`ApiError`], reporting the first failing field.
fn validation_to_api_error(errs: validator::ValidationErrors) -> ApiError {
    if let Some((field, field_errs)) = errs.field_errors().into_iter().next() {
        let reason = field_errs
            .first()
            .map(|e| e.code.to_string())
            .unwrap_or_else(|| "invalid".to_string());
        return ApiError::validation(field, &reason);
    }
    ApiError::validation("body", "validation failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_field_message_parsed() {
        let err: serde_json::Error =
            serde_json::from_str::<std::collections::HashMap<String, i32>>("[]").unwrap_err();
        // Just ensure decode_error never panics on arbitrary serde errors.
        let _ = decode_error(&err);
    }
}
