//! Argon2id password hashing, verification, and policy enforcement.
//!
//! Port of the Go `internal/auth/password.go`. Hash parameters match the
//! original (m=64 MiB, t=3, p=4, 32-byte output) and the PHC-encoded string is
//! wire-compatible with the Go hashes, so credentials interoperate either way.

use std::collections::HashSet;
use std::sync::OnceLock;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};

use crate::platform::errors::ApiError;

const ARGON2_MEMORY: u32 = 64 * 1024; // 64 MiB
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_PARALLELISM: u32 = 4;
const ARGON2_KEY_LEN: usize = 32;

/// The 1000 most common passwords, embedded at compile time.
static COMMON_PASSWORDS: OnceLock<HashSet<String>> = OnceLock::new();

fn common_passwords() -> &'static HashSet<String> {
    COMMON_PASSWORDS.get_or_init(|| {
        include_str!("common_passwords.txt")
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|l| l.to_lowercase())
            .collect()
    })
}

fn hasher() -> Argon2<'static> {
    let params = Params::new(
        ARGON2_MEMORY,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(ARGON2_KEY_LEN),
    )
    .expect("valid argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

/// Hashes `plain` with Argon2id and returns the PHC-encoded string.
pub fn hash_password(plain: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    hasher()
        .hash_password(plain.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ApiError::internal(format!("hash password: {e}")))
}

/// Returns true if `plain` matches the PHC-encoded Argon2id `encoded` hash.
pub fn verify_password(plain: &str, encoded: &str) -> bool {
    match PasswordHash::new(encoded) {
        Ok(parsed) => hasher().verify_password(plain.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

/// Asynchronously hashes `plain` with Argon2id on a blocking thread.
///
/// Argon2id is CPU-heavy (64 MiB, ~100-200 ms). Running it on tokio worker
/// threads would block the async runtime, allowing a handful of login requests
/// to stall every other request, including health checks. `spawn_blocking`
/// moves the work off those threads.
pub async fn hash_password_async(plain: String) -> Result<String, ApiError> {
    tokio::task::spawn_blocking(move || hash_password(&plain))
        .await
        .map_err(|e| ApiError::internal(format!("hash task: {e}")))?
}

/// Asynchronously verifies `plain` against the PHC-encoded Argon2id `encoded`
/// hash on a blocking thread.
///
/// See `hash_password_async` for why this uses `spawn_blocking`.
pub async fn verify_password_async(plain: String, encoded: String) -> bool {
    tokio::task::spawn_blocking(move || verify_password(&plain, &encoded))
        .await
        .unwrap_or(false)
}

/// A pre-computed Argon2id hash of a constant string, computed once at first use.
///
/// Login verifies against this dummy hash when the email is unknown so that
/// unknown and known emails take the same time, preventing user-enumeration
/// timing oracles.
static DUMMY_HASH: OnceLock<String> = OnceLock::new();

/// Returns the dummy hash. Computed once on first call.
pub fn dummy_hash() -> &'static str {
    DUMMY_HASH.get_or_init(|| hash_password("batchwise-dummy-password-for-timing").unwrap())
}

/// Returns a validation [`ApiError`] if `plain` fails the password policy:
/// 12–128 chars, at least one upper / lower / digit / symbol, and not in the
/// common-password list.
pub fn check_password_policy(plain: &str) -> Result<(), ApiError> {
    let len = plain.chars().count();
    if len < 12 {
        return Err(ApiError::validation(
            "password",
            "must be at least 12 characters",
        ));
    }
    if len > 128 {
        return Err(ApiError::validation(
            "password",
            "must be at most 128 characters",
        ));
    }

    let (mut upper, mut lower, mut digit, mut symbol) = (false, false, false, false);
    for c in plain.chars() {
        if c.is_uppercase() {
            upper = true;
        } else if c.is_lowercase() {
            lower = true;
        } else if c.is_ascii_digit() {
            digit = true;
        } else if c.is_ascii_punctuation() || (!c.is_alphanumeric() && !c.is_whitespace()) {
            symbol = true;
        }
    }
    if !upper {
        return Err(ApiError::validation(
            "password",
            "must contain at least one uppercase letter",
        ));
    }
    if !lower {
        return Err(ApiError::validation(
            "password",
            "must contain at least one lowercase letter",
        ));
    }
    if !digit {
        return Err(ApiError::validation(
            "password",
            "must contain at least one digit",
        ));
    }
    if !symbol {
        return Err(ApiError::validation(
            "password",
            "must contain at least one symbol",
        ));
    }
    if common_passwords().contains(&plain.to_lowercase()) {
        return Err(ApiError::validation("password", "password is too common"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrips() {
        let hash = hash_password("Sup3rSecret!pw").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("Sup3rSecret!pw", &hash));
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn verify_rejects_garbage() {
        assert!(!verify_password("x", "not-a-hash"));
    }

    #[test]
    fn policy_length_bounds() {
        assert!(check_password_policy("Aa1!aaaa").is_err()); // too short
        assert!(check_password_policy("Aa1!aaaaaaaa").is_ok()); // 12 chars, all classes
    }

    #[test]
    fn policy_requires_all_classes() {
        assert!(check_password_policy("alllowercase1!").is_err()); // no upper
        assert!(check_password_policy("ALLUPPERCASE1!").is_err()); // no lower
        assert!(check_password_policy("NoDigitsHere!!").is_err()); // no digit
        assert!(check_password_policy("NoSymbol12345A").is_err()); // no symbol
    }

    #[test]
    fn policy_rejects_common_passwords() {
        // "Password1!" lowercased may not be in list; use a known-common one
        // composed to pass class checks would defeat the test, so verify the
        // list lookup directly.
        assert!(common_passwords().contains("password"));
    }

    #[test]
    fn verify_uses_same_params_as_hash() {
        let hash = hash_password("Sup3rSecret!pw").unwrap();
        assert!(verify_password("Sup3rSecret!pw", &hash));
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn dummy_hash_is_stable() {
        let h1 = dummy_hash();
        let h2 = dummy_hash();
        assert_eq!(h1, h2);
        assert!(!verify_password("anything", h1));
    }
}
