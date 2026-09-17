//! HS256 JWT issuance and verification.
//!
//! Port of the Go `internal/auth/jwt.go` (golang-jwt → jsonwebtoken). Claims,
//! issuer/audience validation, 30s leeway, and HS256-only enforcement match the
//! original. Each token also carries a unique `jti` and the user's token version
//! (`ver`), which [`RoleCache`](crate::platform::roles::RoleCache) checks so a
//! token can be revoked before it expires.

use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT payload. `aud` is serialised as an array to match the Go issuer.
#[derive(Debug, Serialize, Deserialize)]
struct TokenClaims {
    sub: String,
    tenant_id: String,
    iss: String,
    aud: Vec<String>,
    exp: i64,
    nbf: i64,
    iat: i64,
    jti: String,
    ver: i32,
}

/// Parsed, validated claims of interest to the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Claims {
    pub subject: Uuid,
    pub tenant_id: Uuid,
    /// Unique token id, used to revoke this one token.
    pub jti: Uuid,
    /// The user's token version when the token was issued.
    pub version: i32,
    pub expires_at: DateTime<Utc>,
}

/// Issues and verifies HS256 access tokens.
#[derive(Clone)]
pub struct Jwt {
    encoding: EncodingKey,
    decoding: DecodingKey,
    issuer: String,
    audience: String,
    ttl_minutes: i64,
}

/// Error verifying a token.
#[derive(Debug, thiserror::Error)]
#[error("invalid token: {0}")]
pub struct VerifyError(String);

impl Jwt {
    /// Builds the issuer/verifier from the shared secret and claims config.
    pub fn new(secret: &str, issuer: &str, audience: &str, ttl_minutes: i64) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
            issuer: issuer.to_string(),
            audience: audience.to_string(),
            ttl_minutes: if ttl_minutes <= 0 { 15 } else { ttl_minutes },
        }
    }

    /// Issues a signed token for `user_id`/`tenant_id` at the user's token
    /// `version`. Returns the token and the number of seconds until it expires.
    pub fn issue(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        version: i32,
    ) -> Result<(String, i64), VerifyError> {
        let now = Utc::now();
        let exp = now + chrono::Duration::minutes(self.ttl_minutes);
        let claims = TokenClaims {
            sub: user_id.to_string(),
            tenant_id: tenant_id.to_string(),
            iss: self.issuer.clone(),
            aud: vec![self.audience.clone()],
            exp: exp.timestamp(),
            nbf: now.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
            ver: version,
        };
        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|e| VerifyError(e.to_string()))?;
        Ok((token, (exp - now).num_seconds()))
    }

    /// Parses and validates a token, returning its [`Claims`].
    pub fn verify(&self, token: &str) -> Result<Claims, VerifyError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.leeway = 30;
        validation.set_required_spec_claims(&["exp"]);

        let data = decode::<TokenClaims>(token, &self.decoding, &validation)
            .map_err(|e| VerifyError(e.to_string()))?;
        let subject =
            Uuid::parse_str(&data.claims.sub).map_err(|e| VerifyError(format!("sub: {e}")))?;
        let tenant_id = Uuid::parse_str(&data.claims.tenant_id)
            .map_err(|e| VerifyError(format!("tenant_id: {e}")))?;
        let jti =
            Uuid::parse_str(&data.claims.jti).map_err(|e| VerifyError(format!("jti: {e}")))?;
        let expires_at = DateTime::from_timestamp(data.claims.exp, 0)
            .ok_or_else(|| VerifyError("exp out of range".into()))?;
        Ok(Claims {
            subject,
            tenant_id,
            jti,
            version: data.claims.ver,
            expires_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jwt() -> Jwt {
        Jwt::new(
            "test-secret-at-least-32-bytes-long!!",
            "batchwise",
            "batchwise",
            15,
        )
    }

    #[test]
    fn issue_then_verify_roundtrips() {
        let (uid, tid) = (Uuid::new_v4(), Uuid::new_v4());
        let (token, expires_in) = jwt().issue(uid, tid, 3).unwrap();
        assert!(expires_in > 0 && expires_in <= 15 * 60);
        let claims = jwt().verify(&token).unwrap();
        assert_eq!(claims.subject, uid);
        assert_eq!(claims.tenant_id, tid);
        assert_eq!(claims.version, 3);
        assert!(claims.expires_at > Utc::now());
    }

    #[test]
    fn every_token_gets_its_own_jti() {
        let (uid, tid) = (Uuid::new_v4(), Uuid::new_v4());
        let (a, _) = jwt().issue(uid, tid, 0).unwrap();
        let (b, _) = jwt().issue(uid, tid, 0).unwrap();
        assert_ne!(jwt().verify(&a).unwrap().jti, jwt().verify(&b).unwrap().jti);
    }

    #[test]
    fn rejects_tokens_without_jti_or_version() {
        // Tokens issued before revocation existed have neither claim.
        let now = Utc::now().timestamp();
        let old = serde_json::json!({
            "sub": Uuid::new_v4().to_string(),
            "tenant_id": Uuid::new_v4().to_string(),
            "iss": "batchwise",
            "aud": ["batchwise"],
            "exp": now + 600,
            "nbf": now,
            "iat": now,
        });
        let token = encode(
            &Header::new(Algorithm::HS256),
            &old,
            &EncodingKey::from_secret(b"test-secret-at-least-32-bytes-long!!"),
        )
        .unwrap();
        assert!(jwt().verify(&token).is_err());
    }

    #[test]
    fn rejects_wrong_secret() {
        let (token, _) = jwt().issue(Uuid::new_v4(), Uuid::new_v4(), 0).unwrap();
        let other = Jwt::new(
            "a-totally-different-secret-key-here!!",
            "batchwise",
            "batchwise",
            15,
        );
        assert!(other.verify(&token).is_err());
    }

    #[test]
    fn rejects_wrong_issuer() {
        let (token, _) = jwt().issue(Uuid::new_v4(), Uuid::new_v4(), 0).unwrap();
        let other = Jwt::new(
            "test-secret-at-least-32-bytes-long!!",
            "evil",
            "batchwise",
            15,
        );
        assert!(other.verify(&token).is_err());
    }

    #[test]
    fn rejects_garbage() {
        assert!(jwt().verify("not.a.jwt").is_err());
    }
}
