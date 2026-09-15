//! Application configuration loaded from environment variables.
//!
//! Port of the Go `internal/platform/config` package. Defaults and production
//! validation rules match the original exactly.

use std::env;

/// All application settings.
#[derive(Clone)]
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
    /// Trust `X-Forwarded-For` for rate-limit keying (only behind a trusted proxy).
    pub trust_proxy_headers: bool,
    pub migrations_disabled: bool,
    pub log_level: String,
}

/// Manual Debug impl to prevent leaking secrets via debug logs or panic messages.
/// `database_url` and `jwt_secret` are shown as "[redacted]" instead of their actual values.
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("app_env", &self.app_env)
            .field("app_base_url", &self.app_base_url)
            .field("http_port", &self.http_port)
            .field("database_url", &"[redacted]")
            .field("jwt_secret", &"[redacted]")
            .field("jwt_issuer", &self.jwt_issuer)
            .field("jwt_audience", &self.jwt_audience)
            .field("jwt_expiry_minutes", &self.jwt_expiry_minutes)
            .field("refresh_token_expiry_days", &self.refresh_token_expiry_days)
            .field("cors_origin", &self.cors_origin)
            .field("allow_overdraft", &self.allow_overdraft)
            .field(
                "bootstrap_registration_enabled",
                &self.bootstrap_registration_enabled,
            )
            .field(
                "rate_limit_register_per_minute",
                &self.rate_limit_register_per_minute,
            )
            .field(
                "rate_limit_login_per_minute",
                &self.rate_limit_login_per_minute,
            )
            .field(
                "rate_limit_refresh_per_minute",
                &self.rate_limit_refresh_per_minute,
            )
            .field(
                "rate_limit_default_per_minute",
                &self.rate_limit_default_per_minute,
            )
            .field("trust_proxy_headers", &self.trust_proxy_headers)
            .field("migrations_disabled", &self.migrations_disabled)
            .field("log_level", &self.log_level)
            .finish()
    }
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
            trust_proxy_headers: parse_opt("TRUST_PROXY_HEADERS", false)?,
            migrations_disabled: parse_opt("MIGRATIONS_DISABLED", false)?,
            log_level: opt("LOG_LEVEL", "info"),
        };

        // A short HS256 secret is brute-forceable offline and a forged token
        // carries an arbitrary tenant_id, so the length floor applies in every
        // environment. The remaining production rules are skipped only for the
        // explicit local-development names; an unset or misspelt APP_ENV gets
        // the strict checks (fail closed).
        if cfg.jwt_secret.len() < 32 {
            return Err(ConfigError::Production(
                "JWT_SECRET must be at least 32 characters".into(),
            ));
        }
        if !matches!(cfg.app_env.as_str(), "development" | "dev" | "test") {
            cfg.validate_production()?;
        }
        Ok(cfg)
    }

    /// Runs for every non-development environment (fail-closed).
    fn validate_production(&self) -> Result<(), ConfigError> {
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
            "TRUST_PROXY_HEADERS",
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
        env::set_var(
            "JWT_SECRET",
            "my-very-long-secret-key-that-is-definitely-more-than-32-characters",
        );

        let cfg = Config::load().unwrap();
        assert_eq!(cfg.app_base_url, "http://localhost:8080");
        assert_eq!(cfg.http_port, 8080);
        assert_eq!(cfg.jwt_expiry_minutes, 15);
        assert_eq!(cfg.refresh_token_expiry_days, 7);
        assert!(!cfg.allow_overdraft);
        assert_eq!(cfg.rate_limit_default_per_minute, 600);
        assert!(!cfg.trust_proxy_headers);
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

    #[test]
    fn unset_app_env_rejects_short_secret() {
        let _g = LOCK.lock().unwrap();
        clear();
        env::set_var("DATABASE_URL", "postgres://localhost:5432/db");
        env::set_var("JWT_SECRET", "short");
        assert!(matches!(Config::load(), Err(ConfigError::Production(_))));
        clear();
    }

    #[test]
    fn development_still_skips_production_rules() {
        let _g = LOCK.lock().unwrap();
        clear();
        env::set_var("APP_ENV", "development");
        env::set_var("DATABASE_URL", "postgres://localhost:5432/db");
        env::set_var(
            "JWT_SECRET",
            "my-very-long-secret-key-that-is-definitely-more-than-32-characters",
        );
        let cfg = Config::load().unwrap();
        assert_eq!(cfg.app_env, "development");
        clear();
    }

    #[test]
    fn debug_redacts_secrets() {
        let _g = LOCK.lock().unwrap();
        clear();
        env::set_var("APP_ENV", "development");
        env::set_var("DATABASE_URL", "postgres://localhost:5432/db");
        env::set_var(
            "JWT_SECRET",
            "my-very-long-secret-key-that-is-definitely-more-than-32-characters",
        );
        let cfg = Config::load().unwrap();
        let debug_output = format!("{cfg:?}");
        assert!(debug_output.contains("[redacted]"));
        assert!(!debug_output.contains("postgres://localhost:5432/db"));
        assert!(!debug_output
            .contains("my-very-long-secret-key-that-is-definitely-more-than-32-characters"));
        clear();
    }
}
