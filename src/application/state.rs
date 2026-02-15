use std::sync::Arc;

use webauthn_rs::prelude::*;

use crate::application::services::{
    AuthorService, BookService, GenreService, ReadingService, StatsInvalidator, TimelineInvalidator,
};
use crate::domain::repositories::{
    AiUsageRepository, AuthorRepository, BookRepository, CoverSuggestionRepository,
    GenreRepository, ImageRepository, PasskeyCredentialRepository, ReadingRepository,
    RegistrationTokenRepository, SessionRepository, StatsRepository, TimelineEventRepository,
    TokenRepository, UserBookRepository, UserRepository,
};
use crate::infrastructure::backup::BackupService;
use crate::infrastructure::database::Database;
use crate::infrastructure::repositories::ai_usage::SqlAiUsageRepository;
use crate::infrastructure::repositories::books::authors::SqlAuthorRepository;
use crate::infrastructure::repositories::books::books::SqlBookRepository;
use crate::infrastructure::repositories::books::genres::SqlGenreRepository;
use crate::infrastructure::repositories::books::readings::SqlReadingRepository;
use crate::infrastructure::repositories::books::user_books::SqlUserBookRepository;
use crate::infrastructure::repositories::cover_suggestions::SqlCoverSuggestionRepository;
use crate::infrastructure::repositories::images::SqlImageRepository;
use crate::infrastructure::repositories::passkey_credentials::SqlPasskeyCredentialRepository;
use crate::infrastructure::repositories::registration_tokens::SqlRegistrationTokenRepository;
use crate::infrastructure::repositories::sessions::SqlSessionRepository;
use crate::infrastructure::repositories::stats::SqlStatsRepository;
use crate::infrastructure::repositories::timeline_events::SqlTimelineEventRepository;
use crate::infrastructure::repositories::tokens::SqlTokenRepository;
use crate::infrastructure::repositories::users::SqlUserRepository;
use crate::infrastructure::webauthn::ChallengeStore;

/// Configuration for external services and auth — everything that varies
/// between production and test environments. Repos and services are created
/// automatically from the database pool.
pub struct AppStateConfig {
    pub webauthn: Arc<Webauthn>,
    pub insecure_cookies: bool,
    pub openrouter_url: String,
    pub openrouter_api_key: String,
    pub openrouter_model: String,
    pub stats_invalidator: StatsInvalidator,
    pub timeline_invalidator: TimelineInvalidator,
}

#[derive(Clone)]
pub struct AppState {
    pub author_repo: Arc<dyn AuthorRepository>,
    pub book_repo: Arc<dyn BookRepository>,
    pub genre_repo: Arc<dyn GenreRepository>,
    pub reading_repo: Arc<dyn ReadingRepository>,
    pub user_book_repo: Arc<dyn UserBookRepository>,
    pub timeline_repo: Arc<dyn TimelineEventRepository>,
    pub user_repo: Arc<dyn UserRepository>,
    pub token_repo: Arc<dyn TokenRepository>,
    pub session_repo: Arc<dyn SessionRepository>,
    pub passkey_repo: Arc<dyn PasskeyCredentialRepository>,
    pub registration_token_repo: Arc<dyn RegistrationTokenRepository>,
    pub ai_usage_repo: Arc<dyn AiUsageRepository>,
    pub image_repo: Arc<dyn ImageRepository>,
    pub cover_suggestion_repo: Arc<dyn CoverSuggestionRepository>,
    pub stats_repo: Arc<dyn StatsRepository>,
    pub webauthn: Arc<Webauthn>,
    pub challenge_store: Arc<ChallengeStore>,
    pub http_client: reqwest::Client,
    pub openrouter_url: String,
    pub openrouter_api_key: String,
    pub openrouter_model: String,
    pub backup_service: Arc<BackupService>,
    pub author_service: AuthorService,
    pub genre_service: GenreService,
    pub book_service: BookService,
    pub reading_service: ReadingService,
    pub insecure_cookies: bool,
    pub stats_invalidator: StatsInvalidator,
    pub timeline_invalidator: TimelineInvalidator,
    pub image_semaphore: Arc<tokio::sync::Semaphore>,
}

impl AppState {
    /// Build the full application state from a database connection and config.
    /// Creates all repositories and services internally.
    pub fn from_database(database: &Database, config: AppStateConfig) -> Self {
        let pool = database.clone_pool();

        let author_repo: Arc<dyn AuthorRepository> =
            Arc::new(SqlAuthorRepository::new(pool.clone()));
        let book_repo: Arc<dyn BookRepository> = Arc::new(SqlBookRepository::new(pool.clone()));
        let genre_repo: Arc<dyn GenreRepository> = Arc::new(SqlGenreRepository::new(pool.clone()));
        let reading_repo: Arc<dyn ReadingRepository> =
            Arc::new(SqlReadingRepository::new(pool.clone()));
        let user_book_repo: Arc<dyn UserBookRepository> =
            Arc::new(SqlUserBookRepository::new(pool.clone()));
        let timeline_repo: Arc<dyn TimelineEventRepository> =
            Arc::new(SqlTimelineEventRepository::new(pool.clone()));
        let user_repo: Arc<dyn UserRepository> = Arc::new(SqlUserRepository::new(pool.clone()));
        let token_repo: Arc<dyn TokenRepository> = Arc::new(SqlTokenRepository::new(pool.clone()));
        let session_repo: Arc<dyn SessionRepository> =
            Arc::new(SqlSessionRepository::new(pool.clone()));
        let passkey_repo: Arc<dyn PasskeyCredentialRepository> =
            Arc::new(SqlPasskeyCredentialRepository::new(pool.clone()));
        let registration_token_repo: Arc<dyn RegistrationTokenRepository> =
            Arc::new(SqlRegistrationTokenRepository::new(pool.clone()));
        let ai_usage_repo: Arc<dyn AiUsageRepository> =
            Arc::new(SqlAiUsageRepository::new(pool.clone()));
        let image_repo: Arc<dyn ImageRepository> = Arc::new(SqlImageRepository::new(pool.clone()));
        let cover_suggestion_repo: Arc<dyn CoverSuggestionRepository> =
            Arc::new(SqlCoverSuggestionRepository::new(pool.clone()));
        let stats_repo: Arc<dyn StatsRepository> = Arc::new(SqlStatsRepository::new(pool.clone()));

        let backup_service = Arc::new(BackupService::new(pool));

        let author_service =
            AuthorService::new(Arc::clone(&author_repo), Arc::clone(&timeline_repo));
        let genre_service = GenreService::new(Arc::clone(&genre_repo), Arc::clone(&timeline_repo));
        let book_service = BookService::new(
            Arc::clone(&book_repo),
            Arc::clone(&author_repo),
            Arc::clone(&genre_repo),
            Arc::clone(&timeline_repo),
        );
        let reading_service = ReadingService::new(
            Arc::clone(&reading_repo),
            Arc::clone(&book_repo),
            Arc::clone(&author_repo),
            Arc::clone(&timeline_repo),
            Arc::clone(&user_book_repo),
        );
        Self {
            author_repo,
            book_repo,
            genre_repo,
            reading_repo,
            user_book_repo,
            timeline_repo,
            user_repo,
            token_repo,
            session_repo,
            passkey_repo,
            registration_token_repo,
            ai_usage_repo,
            image_repo,
            cover_suggestion_repo,
            stats_repo,
            webauthn: config.webauthn,
            challenge_store: Arc::new(ChallengeStore::new()),
            #[allow(clippy::expect_used)]
            http_client: reqwest::ClientBuilder::new()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build HTTP client"),
            openrouter_url: config.openrouter_url,
            openrouter_api_key: config.openrouter_api_key,
            openrouter_model: config.openrouter_model,
            backup_service,
            author_service,
            genre_service,
            book_service,
            reading_service,
            insecure_cookies: config.insecure_cookies,
            stats_invalidator: config.stats_invalidator,
            timeline_invalidator: config.timeline_invalidator,
            image_semaphore: Arc::new(tokio::sync::Semaphore::new(4)),
        }
    }
}
