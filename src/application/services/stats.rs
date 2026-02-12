use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tracing::{error, info};

use crate::domain::ids::UserId;
use crate::domain::repositories::StatsRepository;
use crate::domain::stats::CachedStats;

/// Sends invalidation signals to the background stats recomputer.
/// Non-blocking and fire-and-forget — safe to call from any handler.
#[derive(Clone)]
pub struct StatsInvalidator {
    tx: mpsc::Sender<UserId>,
}

impl StatsInvalidator {
    pub fn new(tx: mpsc::Sender<UserId>) -> Self {
        Self { tx }
    }

    /// Signal that stats need recomputation for the given user.
    pub fn invalidate(&self, user_id: UserId) {
        let _ = self.tx.try_send(user_id);
    }
}

/// Listens for invalidation signals, debounces, and recomputes all stats.
/// Runs as a long-lived background task — spawn with `tokio::spawn`.
pub async fn stats_recomputation_task(
    mut rx: mpsc::Receiver<UserId>,
    stats_repo: Arc<dyn StatsRepository>,
    debounce: Duration,
) {
    loop {
        let Some(first_user_id) = rx.recv().await else {
            break;
        };

        // Debounce: wait then drain any accumulated signals, collecting unique user IDs
        let mut user_ids = HashSet::new();
        user_ids.insert(first_user_id);
        tokio::time::sleep(debounce).await;
        while let Ok(uid) = rx.try_recv() {
            user_ids.insert(uid);
        }

        for user_id in user_ids {
            match compute_all_stats(&*stats_repo, user_id).await {
                Ok(cached) => {
                    if let Err(err) = stats_repo.store_cached(user_id, &cached).await {
                        error!(error = %err, %user_id, "failed to store stats cache");
                    }
                }
                Err(err) => error!(error = %err, %user_id, "stats recomputation failed"),
            }
        }
    }
}

/// Runs all stats queries and assembles a complete `CachedStats` snapshot.
/// Logs the total computation time on success.
pub async fn compute_all_stats(
    repo: &dyn StatsRepository,
    user_id: UserId,
) -> Result<CachedStats, crate::domain::RepositoryError> {
    let start = Instant::now();

    let (book_summary, reading) =
        tokio::join!(repo.book_summary(user_id), repo.reading_summary(user_id),);

    let cached = CachedStats {
        book_summary: book_summary?,
        reading: reading?,
        computed_at: chrono::Utc::now().to_rfc3339(),
    };

    info!(duration_ms = start.elapsed().as_millis(), %user_id, "stats computed");
    Ok(cached)
}

/// Compute stats for a specific year. Not cached — runs on demand.
pub async fn compute_stats_for_year(
    repo: &dyn StatsRepository,
    user_id: UserId,
    year: i32,
) -> Result<CachedStats, crate::domain::RepositoryError> {
    let start = Instant::now();

    let (book_summary, reading) = tokio::join!(
        repo.book_summary_for_year(user_id, year),
        repo.reading_summary_for_year(user_id, year),
    );

    let cached = CachedStats {
        book_summary: book_summary?,
        reading: reading?,
        computed_at: chrono::Utc::now().to_rfc3339(),
    };

    info!(duration_ms = start.elapsed().as_millis(), %user_id, year, "year stats computed");
    Ok(cached)
}
