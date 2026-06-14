//! `tracing` subscriber setup.
//!
//! Port of the Go `internal/platform/logger` package. JSON output in
//! production, human-readable text otherwise; level taken from config.

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialises the global tracing subscriber. Safe to call once at startup.
pub fn init(app_env: &str, log_level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("batchwise={level},info", level = log_level)));

    let registry = tracing_subscriber::registry().with(filter);

    if app_env == "production" {
        registry
            .with(fmt::layer().json().with_current_span(true))
            .init();
    } else {
        registry.with(fmt::layer().with_target(false)).init();
    }
}
