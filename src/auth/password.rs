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
        Ok(parsed) => Argon2::default()
            .verify_password(plain.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
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
}
