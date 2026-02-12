pub mod analytics;
pub mod auth;
pub mod books;
pub mod cover_suggestions;
pub mod images;
pub(crate) mod macros;
pub mod pagination;

// Re-exports for backward compatibility
pub use analytics::{ai_usage, stats, timeline_events};
pub use auth::{passkey_credentials, registration_tokens, sessions, tokens, users};
pub use books::{authors, books as book_repos, genres, readings, user_books};
