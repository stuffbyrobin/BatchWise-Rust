//! Batchwise server entry point.
//!
//! Wires config, logger, the database pool, and the domain routers into an axum
//! application. The router itself is assembled in [`batchwise::app`].

use std::net::SocketAddr;

use batchwise::app::build_router;
use batchwise::platform::{config::Config, database, logger};
use batchwise::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let cfg = Config::load()?;
    logger::init(&cfg.app_env, &cfg.log_level);

    let pool = database::connect(&cfg.database_url).await?;
    if cfg.migrations_disabled {
        tracing::warn!("migrations disabled via MIGRATIONS_DISABLED");
    } else {
        database::migrate(&pool).await?;
        tracing::info!("migrations applied");
    }

    let http_port = cfg.http_port;
    let state = AppState::new(pool.clone(), cfg);

    batchwise::auth::cleanup::start_cleanup_loop(pool);

    let app = build_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], http_port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "batchwise listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
