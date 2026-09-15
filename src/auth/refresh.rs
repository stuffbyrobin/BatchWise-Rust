//! Opaque refresh-token generation and hashing.
//!
//! Port of the Go `internal/auth/refresh.go`. The token is 256 random bits
//! encoded as unpadded base64url; only its SHA-256 hex digest is stored.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Returns a new opaque token (32 random bytes as unpadded base64url) and its SHA-256 hex hash.
pub fn generate_refresh_token() -> (String, String) {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let token = URL_SAFE_NO_PAD.encode(bytes);
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
        assert_eq!(token.len(), 43); // 32 bytes -> 43 base64url chars
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
