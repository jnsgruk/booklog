use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use chrono::{Duration, Utc};
use tokio::net::TcpListener;
use tokio::signal;
use tracing::info;
use webauthn_rs::prelude::*;

use crate::application::routes::app_router;
use crate::application::services::stats::stats_recomputation_task;
use crate::application::services::timeline_refresh::{TimelineInvalidation, timeline_rebuild_task};
use crate::application::services::{StatsInvalidator, TimelineInvalidator};
use crate::application::state::{AppState, AppStateConfig};
use crate::domain::registration_tokens::NewRegistrationToken;
use crate::domain::repositories::{RegistrationTokenRepository, UserRepository};
use crate::infrastructure::auth::{generate_session_token, hash_token};
use crate::infrastructure::database::Database;

pub struct ServerConfig {
    pub bind_address: SocketAddr,
    pub database_url: String,
    pub rp_id: String,
    pub rp_origin: String,
    pub insecure_cookies: bool,
    pub openrouter_api_key: String,
    pub openrouter_model: String,
}

pub async fn serve(config: ServerConfig) -> anyhow::Result<()> {
    let database = Database::connect(&config.database_url)
        .await
        .context("failed to connect to database")?;

    let rp_origin = url::Url::parse(&config.rp_origin).context("invalid BOOKLOG_RP_ORIGIN URL")?;
    let webauthn = Arc::new(
        WebauthnBuilder::new(&config.rp_id, &rp_origin)
            .context("failed to build WebAuthn instance")?
            .rp_name("Booklog")
            .build()
            .context("failed to build WebAuthn instance")?,
    );

    let (stats_tx, stats_rx) = tokio::sync::mpsc::channel::<crate::domain::ids::UserId>(32);
    let stats_invalidator = StatsInvalidator::new(stats_tx);

    let (timeline_tx, timeline_rx) = tokio::sync::mpsc::channel::<TimelineInvalidation>(32);
    let timeline_invalidator = TimelineInvalidator::new(timeline_tx);

    let state = AppState::from_database(
        &database,
        AppStateConfig {
            webauthn,
            insecure_cookies: config.insecure_cookies,
            openrouter_url: crate::infrastructure::ai::OPENROUTER_URL.to_string(),
            openrouter_api_key: config.openrouter_api_key,
            openrouter_model: config.openrouter_model,
            stats_invalidator: stats_invalidator.clone(),
            timeline_invalidator,
        },
    );

    // Spawn background stats recomputation task
    let stats_repo = Arc::clone(&state.stats_repo);
    tokio::spawn(stats_recomputation_task(
        stats_rx,
        stats_repo,
        std::time::Duration::from_secs(2),
    ));

    // Spawn background timeline rebuild task
    tokio::spawn(timeline_rebuild_task(
        timeline_rx,
        Arc::clone(&state.author_repo),
        Arc::clone(&state.book_repo),
        Arc::clone(&state.genre_repo),
        Arc::clone(&state.reading_repo),
        Arc::clone(&state.timeline_repo),
        std::time::Duration::from_secs(2),
    ));

    // Spawn background cover suggestion cleanup task (hourly, removes >24h old)
    let cover_repo = Arc::clone(&state.cover_suggestion_repo);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            match cover_repo
                .delete_older_than(std::time::Duration::from_secs(86400))
                .await
            {
                Ok(count) if count > 0 => {
                    info!(count, "cleaned up expired cover suggestions");
                }
                Err(err) => {
                    tracing::warn!(error = %err, "cover suggestion cleanup failed");
                }
                _ => {}
            }
        }
    });

    // Seed the stats cache on startup for all existing users
    if let Ok(users) = state.user_repo.list_all().await {
        for user in users {
            stats_invalidator.invalidate(user.id);
        }
    }

    // Clean up expired sessions on startup
    if let Err(err) = state.session_repo.delete_expired().await {
        tracing::warn!(error = %err, "failed to clean up expired sessions on startup");
    }

    // Bootstrap: if no users exist, generate a one-time registration token
    bootstrap_registration(
        &state.registration_token_repo,
        &state.user_repo,
        &config.rp_origin,
    )
    .await?;

    let listener = TcpListener::bind(config.bind_address)
        .await
        .with_context(|| format!("failed to bind to {}", config.bind_address))?;

    let app = app_router(state);

    info!(
        address = %config.bind_address,
        database = %config.database_url,
        "starting HTTP server"
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("server terminated unexpectedly")?;

    info!("server shutdown complete");

    Ok(())
}

async fn bootstrap_registration(
    registration_token_repo: &Arc<dyn RegistrationTokenRepository>,
    user_repo: &Arc<dyn UserRepository>,
    rp_origin: &str,
) -> anyhow::Result<()> {
    let users_exist = user_repo
        .exists()
        .await
        .context("failed to check if users exist")?;

    if users_exist {
        return Ok(());
    }

    // Generate one-time registration token
    let token = generate_session_token();
    let token_hash = hash_token(&token);
    let now = Utc::now();
    #[allow(clippy::expect_used)]
    let expires_at = now
        .checked_add_signed(Duration::hours(1))
        .expect("timestamp overflow adding 1 hour");

    let new_token = NewRegistrationToken::new(token_hash, now, expires_at);

    registration_token_repo
        .insert(new_token)
        .await
        .context("failed to create registration token")?;

    info!("No users found. Register the first user at:");
    info!("  {}/register/{}", rp_origin, token);
    info!("This link expires in 1 hour.");

    Ok(())
}

#[allow(clippy::expect_used)] // Startup: panicking is appropriate if signal handlers fail
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
