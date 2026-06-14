//! Opaque refresh-token generation and hashing.
//!
//! Port of the Go `internal/auth/refresh.go`. The token is a ULID; only its
//! SHA-256 hex digest is stored.

use sha2::{Digest, Sha256};
use ulid::Ulid;

/// Returns a new opaque ULID token and its SHA-256 hex hash.
pub fn generate_refresh_token() -> (String, String) {
    let token = Ulid::new().to_string();
    let hash = hash_refresh_token(&token);
    (token, hash)
}

/// Returns the SHA-256 hex digest of a plaintext refresh token.
pub fn hash_refresh_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex_encode(&digest)
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_and_hash_are_stable() {
        let (token, hash) = generate_refresh_token();
        assert_eq!(token.len(), 26); // ULID
        assert_eq!(hash.len(), 64); // sha256 hex
        assert_eq!(hash_refresh_token(&token), hash);
    }

    #[test]
    fn distinct_tokens() {
        let (a, _) = generate_refresh_token();
        let (b, _) = generate_refresh_token();
        assert_ne!(a, b);
    }
}
