//! Request-scoped values (tenant id, user id, request id, actor id).
//!
//! Port of the Go `internal/platform/context` package. Go threaded these
//! through `context.Context`; in axum they live in the request extensions as a
//! single [`RequestContext`] value, populated by middleware and read by
//! handlers via the [`RequestContext`] extractor.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

use super::errors::ApiError;

/// Values attached to every request as it flows through the middleware stack.
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    pub tenant_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub request_id: String,
    /// Acting user for audit logging; usually equal to `user_id`.
    pub actor_id: Option<Uuid>,
}

impl RequestContext {
    /// Returns the tenant id or an `unauthorized` error if none is set.
    pub fn tenant_id(&self) -> Result<Uuid, ApiError> {
        self.tenant_id
            .ok_or_else(|| ApiError::unauthorized("missing tenant context"))
    }

    /// Returns the user id or an `unauthorized` error if none is set.
    pub fn user_id(&self) -> Result<Uuid, ApiError> {
        self.user_id
            .ok_or_else(|| ApiError::unauthorized("missing user context"))
    }
}

/// Extractor: pulls the [`RequestContext`] cloned out of request extensions.
///
/// Returns an empty context (no ids, blank request id) when middleware has not
/// installed one — matching the Go helpers' "absent" semantics.
impl<S> FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<RequestContext>()
            .cloned()
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_context_has_no_ids() {
        let ctx = RequestContext::default();
        assert!(ctx.tenant_id.is_none());
        assert!(ctx.user_id.is_none());
        assert_eq!(ctx.request_id, "");
        assert!(ctx.tenant_id().is_err());
        assert!(ctx.user_id().is_err());
    }

    #[test]
    fn populated_context_returns_ids() {
        let id = Uuid::new_v4();
        let ctx = RequestContext {
            tenant_id: Some(id),
            user_id: Some(id),
            request_id: "req-1".into(),
            actor_id: Some(id),
        };
        assert_eq!(ctx.tenant_id().unwrap(), id);
        assert_eq!(ctx.user_id().unwrap(), id);
    }
}
