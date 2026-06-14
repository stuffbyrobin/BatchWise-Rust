//! Application configuration loaded from environment variables.
//!
//! Port of the Go `internal/platform/config` package. Defaults and production
//! validation rules match the original exactly.

use std::env;

/// All application settings.
#[derive(Debug, Clone)]
pub struct Config {
    pub app_env: String,
    pub app_base_url: String,
    pub http_port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub jwt_expiry_minutes: i64,
    pub refresh_token_expiry_days: i64,
    pub cors_origin: String,
    pub allow_overdraft: bool,
    pub bootstrap_registration_enabled: bool,
    pub rate_limit_register_per_minute: u32,
    pub rate_limit_login_per_minute: u32,
    pub rate_limit_refresh_per_minute: u32,
    pub rate_limit_default_per_minute: u32,
    pub migrations_disabled: bool,
    pub log_level: String,
}

/// Error returned when configuration cannot be loaded or fails validation.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} is required")]
    Missing(&'static str),
    #[error("{0} is invalid: {1}")]
    Invalid(&'static str, String),
    #[error("{0}")]
    Production(String),
}

impl Config {
    /// Parses environment variables into [`Config`] and validates production constraints.
    pub fn load() -> Result<Self, ConfigError> {
        let cfg = Config {
            app_env: opt("APP_ENV", "development"),
            app_base_url: opt("APP_BASE_URL", "http://localhost:8080"),
            http_port: parse_opt("HTTP_PORT", 8080)?,
            database_url: required("DATABASE_URL")?,
            jwt_secret: required("JWT_SECRET")?,
            jwt_issuer: opt("JWT_ISSUER", "batchwise"),
            jwt_audience: opt("JWT_AUDIENCE", "batchwise"),
            jwt_expiry_minutes: parse_opt("JWT_EXPIRY_MINUTES", 15)?,
            refresh_token_expiry_days: parse_opt("REFRESH_TOKEN_EXPIRY_DAYS", 7)?,
            cors_origin: opt("CORS_ORIGIN", "http://localhost:5173"),
            allow_overdraft: parse_opt("ALLOW_OVERDRAFT", false)?,
            bootstrap_registration_enabled: parse_opt("BOOTSTRAP_REGISTRATION_ENABLED", false)?,
            rate_limit_register_per_minute: parse_opt("RATE_LIMIT_REGISTER_PER_MINUTE", 5)?,
            rate_limit_login_per_minute: parse_opt("RATE_LIMIT_LOGIN_PER_MINUTE", 10)?,
            rate_limit_refresh_per_minute: parse_opt("RATE_LIMIT_REFRESH_PER_MINUTE", 30)?,
            rate_limit_default_per_minute: parse_opt("RATE_LIMIT_DEFAULT_PER_MINUTE", 600)?,
            migrations_disabled: parse_opt("MIGRATIONS_DISABLED", false)?,
            log_level: opt("LOG_LEVEL", "info"),
        };

        if cfg.app_env == "production" {
            cfg.validate_production()?;
        }
        Ok(cfg)
    }

    fn validate_production(&self) -> Result<(), ConfigError> {
        if self.jwt_secret.len() < 32 {
            return Err(ConfigError::Production(
                "JWT_SECRET must be at least 32 characters in production".into(),
            ));
        }
        if self.cors_origin.contains('*') {
            return Err(ConfigError::Production(
                "CORS_ORIGIN cannot contain '*' in production".into(),
            ));
        }
        if !self.app_base_url.starts_with("https://") {
            return Err(ConfigError::Production(
                "APP_BASE_URL must start with 'https://' in production".into(),
            ));
        }
        if self.jwt_issuer == "batchwise" {
            return Err(ConfigError::Production(
                "JWT_ISSUER cannot be default 'batchwise' in production".into(),
            ));
        }
        if self.jwt_audience == "batchwise" {
            return Err(ConfigError::Production(
                "JWT_AUDIENCE cannot be default 'batchwise' in production".into(),
            ));
        }
        Ok(())
    }
}

fn required(key: &'static str) -> Result<String, ConfigError> {
    match env::var(key) {
        Ok(v) if !v.is_empty() => Ok(v),
        _ => Err(ConfigError::Missing(key)),
    }
}

fn opt(key: &str, default: &str) -> String {
    env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn parse_opt<T>(key: &'static str, default: T) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(v) if !v.is_empty() => v
            .parse::<T>()
            .map_err(|e| ConfigError::Invalid(key, e.to_string())),
        _ => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Env access is process-global; serialise these tests with a mutex.
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn clear() {
        for k in [
            "APP_ENV",
            "APP_BASE_URL",
            "HTTP_PORT",
            "DATABASE_URL",
            "JWT_SECRET",
            "JWT_ISSUER",
            "JWT_AUDIENCE",
            "CORS_ORIGIN",
        ] {
            env::remove_var(k);
        }
    }

    #[test]
    fn loads_defaults_with_required_set() {
        let _g = LOCK.lock().unwrap();
        clear();
        env::set_var("APP_ENV", "development");
        env::set_var("DATABASE_URL", "postgres://localhost:5432/db");
        env::set_var("JWT_SECRET", "my-secret-key");

        let cfg = Config::load().unwrap();
        assert_eq!(cfg.app_base_url, "http://localhost:8080");
        assert_eq!(cfg.http_port, 8080);
        assert_eq!(cfg.jwt_expiry_minutes, 15);
        assert_eq!(cfg.refresh_token_expiry_days, 7);
        assert!(!cfg.allow_overdraft);
        assert_eq!(cfg.rate_limit_default_per_minute, 600);
        clear();
    }

    #[test]
    fn missing_required_errors() {
        let _g = LOCK.lock().unwrap();
        clear();
        assert!(matches!(
            Config::load(),
            Err(ConfigError::Missing("DATABASE_URL"))
        ));
        clear();
    }

    #[test]
    fn production_rejects_short_secret() {
        let _g = LOCK.lock().unwrap();
        clear();
        env::set_var("APP_ENV", "production");
        env::set_var("DATABASE_URL", "postgres://localhost:5432/db");
        env::set_var("JWT_SECRET", "short");
        env::set_var("APP_BASE_URL", "https://example.com");
        env::set_var("JWT_ISSUER", "acme");
        env::set_var("JWT_AUDIENCE", "acme");
        assert!(matches!(Config::load(), Err(ConfigError::Production(_))));
        clear();
    }
}
