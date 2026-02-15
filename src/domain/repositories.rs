use std::collections::HashSet;

use std::time::Duration;

use super::RepositoryError;
use crate::domain::ai_usage::{AiUsage, AiUsageSummary, NewAiUsage};
use crate::domain::cover_suggestions::CoverSuggestion;
use crate::domain::listing::{ListRequest, Page, SortDirection, SortKey};

use crate::domain::authors::{Author, AuthorSortKey, NewAuthor, UpdateAuthor};
use crate::domain::book_items::{Book, BookSortKey, BookWithAuthors, NewBook, UpdateBook};
use crate::domain::genres::{Genre, GenreSortKey, NewGenre, UpdateGenre};
use crate::domain::ids::{
    AuthorId, BookId, GenreId, PasskeyCredentialId, ReadingId, RegistrationTokenId, SessionId,
    TokenId, UserBookId, UserId,
};
use crate::domain::images::EntityImage;
use crate::domain::passkey_credentials::{NewPasskeyCredential, PasskeyCredential};
use crate::domain::readings::{
    NewReading, Reading, ReadingFilter, ReadingSortKey, ReadingWithBook, UpdateReading,
};
use crate::domain::registration_tokens::{NewRegistrationToken, RegistrationToken};
use crate::domain::sessions::{NewSession, Session};
use crate::domain::timeline::{NewTimelineEvent, TimelineEvent, TimelineSortKey};
use crate::domain::tokens::{NewToken, Token};
use crate::domain::user_books::{
    NewUserBook, Shelf, UserBook, UserBookSortKey, UserBookWithDetails,
};
use crate::domain::users::{NewUser, User};
use async_trait::async_trait;

#[async_trait]
pub trait AuthorRepository: Send + Sync {
    async fn insert(&self, author: NewAuthor) -> Result<Author, RepositoryError>;
    async fn get(&self, id: AuthorId) -> Result<Author, RepositoryError>;
    async fn get_by_name(&self, name: &str) -> Result<Author, RepositoryError>;
    async fn list(
        &self,
        request: &ListRequest<AuthorSortKey>,
        search: Option<&str>,
    ) -> Result<Page<Author>, RepositoryError>;
    /// List authors whose books appear in the given user's library.
    async fn list_for_user_library(
        &self,
        user_id: UserId,
        request: &ListRequest<AuthorSortKey>,
        search: Option<&str>,
    ) -> Result<Page<Author>, RepositoryError>;
    async fn update(&self, id: AuthorId, changes: UpdateAuthor) -> Result<Author, RepositoryError>;
    async fn delete(&self, id: AuthorId) -> Result<(), RepositoryError>;

    async fn list_all(&self) -> Result<Vec<Author>, RepositoryError> {
        let sort_key = <AuthorSortKey as SortKey>::default();
        let request =
            ListRequest::<AuthorSortKey>::show_all(sort_key, sort_key.default_direction());
        let page = self.list(&request, None).await?;
        Ok(page.items)
    }

    async fn list_all_sorted(
        &self,
        sort_key: AuthorSortKey,
        direction: SortDirection,
    ) -> Result<Vec<Author>, RepositoryError> {
        let request = ListRequest::show_all(sort_key, direction);
        let page = self.list(&request, None).await?;
        Ok(page.items)
    }
}

#[async_trait]
pub trait GenreRepository: Send + Sync {
    async fn insert(&self, genre: NewGenre) -> Result<Genre, RepositoryError>;
    async fn get(&self, id: GenreId) -> Result<Genre, RepositoryError>;
    async fn get_by_name(&self, name: &str) -> Result<Genre, RepositoryError>;
    async fn list(
        &self,
        request: &ListRequest<GenreSortKey>,
        search: Option<&str>,
    ) -> Result<Page<Genre>, RepositoryError>;
    async fn update(&self, id: GenreId, changes: UpdateGenre) -> Result<Genre, RepositoryError>;
    async fn delete(&self, id: GenreId) -> Result<(), RepositoryError>;

    async fn list_all(&self) -> Result<Vec<Genre>, RepositoryError> {
        let sort_key = <GenreSortKey as SortKey>::default();
        let request = ListRequest::<GenreSortKey>::show_all(sort_key, sort_key.default_direction());
        let page = self.list(&request, None).await?;
        Ok(page.items)
    }

    async fn list_all_sorted(
        &self,
        sort_key: GenreSortKey,
        direction: SortDirection,
    ) -> Result<Vec<Genre>, RepositoryError> {
        let request = ListRequest::show_all(sort_key, direction);
        let page = self.list(&request, None).await?;
        Ok(page.items)
    }
}

#[async_trait]
pub trait BookRepository: Send + Sync {
    async fn insert(&self, book: NewBook) -> Result<Book, RepositoryError>;
    async fn get(&self, id: BookId) -> Result<Book, RepositoryError>;
    async fn get_with_authors(&self, id: BookId) -> Result<BookWithAuthors, RepositoryError>;
    async fn get_by_title(&self, title: &str) -> Result<Book, RepositoryError>;
    async fn get_by_isbn(&self, isbn: &str) -> Result<Book, RepositoryError>;
    async fn list(
        &self,
        request: &ListRequest<BookSortKey>,
        search: Option<&str>,
    ) -> Result<Page<BookWithAuthors>, RepositoryError>;
    async fn list_by_author(
        &self,
        author_id: AuthorId,
    ) -> Result<Vec<BookWithAuthors>, RepositoryError>;
    async fn list_by_genre(
        &self,
        genre_id: GenreId,
    ) -> Result<Vec<BookWithAuthors>, RepositoryError>;
    async fn update(&self, id: BookId, changes: UpdateBook) -> Result<Book, RepositoryError>;
    async fn delete(&self, id: BookId) -> Result<(), RepositoryError>;

    async fn list_all(&self) -> Result<Vec<BookWithAuthors>, RepositoryError> {
        let sort_key = <BookSortKey as SortKey>::default();
        let request = ListRequest::<BookSortKey>::show_all(sort_key, sort_key.default_direction());
        let page = self.list(&request, None).await?;
        Ok(page.items)
    }
}

#[async_trait]
pub trait UserBookRepository: Send + Sync {
    async fn insert(&self, user_book: NewUserBook) -> Result<UserBook, RepositoryError>;
    async fn get(&self, id: UserBookId) -> Result<UserBook, RepositoryError>;
    async fn get_by_user_and_book(
        &self,
        user_id: UserId,
        book_id: BookId,
    ) -> Result<UserBook, RepositoryError>;
    async fn list_by_user(
        &self,
        user_id: UserId,
        shelf: Option<Shelf>,
        request: &ListRequest<UserBookSortKey>,
        search: Option<&str>,
    ) -> Result<Page<UserBookWithDetails>, RepositoryError>;
    async fn move_shelf(&self, id: UserBookId, shelf: Shelf) -> Result<UserBook, RepositoryError>;
    async fn set_book_club(
        &self,
        id: UserBookId,
        book_club: bool,
    ) -> Result<UserBook, RepositoryError>;
    async fn delete(&self, id: UserBookId) -> Result<(), RepositoryError>;
    async fn book_ids_for_user(
        &self,
        user_id: UserId,
        shelf: Option<Shelf>,
    ) -> Result<HashSet<BookId>, RepositoryError>;
}

#[async_trait]
pub trait ReadingRepository: Send + Sync {
    async fn insert(&self, reading: NewReading) -> Result<Reading, RepositoryError>;
    async fn get(&self, id: ReadingId) -> Result<Reading, RepositoryError>;
    async fn get_with_book(&self, id: ReadingId) -> Result<ReadingWithBook, RepositoryError>;
    async fn list(
        &self,
        filter: ReadingFilter,
        request: &ListRequest<ReadingSortKey>,
        search: Option<&str>,
    ) -> Result<Page<ReadingWithBook>, RepositoryError>;
    async fn update(
        &self,
        id: ReadingId,
        changes: UpdateReading,
    ) -> Result<Reading, RepositoryError>;
    async fn delete(&self, id: ReadingId) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait TimelineEventRepository: Send + Sync {
    async fn insert(&self, event: NewTimelineEvent) -> Result<TimelineEvent, RepositoryError>;
    async fn list(
        &self,
        user_id: Option<UserId>,
        request: &ListRequest<TimelineSortKey>,
    ) -> Result<Page<TimelineEvent>, RepositoryError>;

    async fn update_by_entity(
        &self,
        entity_type: &str,
        entity_id: i64,
        title: &str,
        details: &[crate::domain::timeline::TimelineEventDetail],
        genres: &[String],
        reading_data: Option<&crate::domain::timeline::TimelineReadingData>,
    ) -> Result<(), RepositoryError>;

    async fn delete_by_entity(
        &self,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<(), RepositoryError>;

    async fn delete_all(&self) -> Result<(), RepositoryError>;

    async fn list_all(&self) -> Result<Vec<TimelineEvent>, RepositoryError> {
        let sort_key = <TimelineSortKey as SortKey>::default();
        let request =
            ListRequest::<TimelineSortKey>::show_all(sort_key, sort_key.default_direction());
        let page = self.list(None, &request).await?;
        Ok(page.items)
    }
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn insert(&self, user: NewUser) -> Result<User, RepositoryError>;
    async fn get(&self, id: UserId) -> Result<User, RepositoryError>;
    async fn get_by_username(&self, username: &str) -> Result<User, RepositoryError>;
    async fn get_by_uuid(&self, uuid: &str) -> Result<User, RepositoryError>;
    async fn exists(&self) -> Result<bool, RepositoryError>;
    async fn list_all(&self) -> Result<Vec<User>, RepositoryError>;
}

#[async_trait]
pub trait TokenRepository: Send + Sync {
    async fn insert(&self, token: NewToken) -> Result<Token, RepositoryError>;
    async fn get(&self, id: TokenId) -> Result<Token, RepositoryError>;
    async fn get_by_token_hash(&self, token_hash: &str) -> Result<Token, RepositoryError>;
    async fn list_by_user(&self, user_id: UserId) -> Result<Vec<Token>, RepositoryError>;
    async fn revoke(&self, id: TokenId) -> Result<Token, RepositoryError>;
    async fn update_last_used(&self, id: TokenId) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn insert(&self, session: NewSession) -> Result<Session, RepositoryError>;
    async fn get(&self, id: SessionId) -> Result<Session, RepositoryError>;
    async fn get_by_token_hash(&self, token_hash: &str) -> Result<Session, RepositoryError>;
    async fn delete(&self, id: SessionId) -> Result<(), RepositoryError>;
    async fn delete_expired(&self) -> Result<(), RepositoryError>;
    async fn set_acting_as(
        &self,
        id: SessionId,
        acting_as: Option<UserId>,
    ) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait PasskeyCredentialRepository: Send + Sync {
    async fn insert(
        &self,
        credential: NewPasskeyCredential,
    ) -> Result<PasskeyCredential, RepositoryError>;
    async fn get(&self, id: PasskeyCredentialId) -> Result<PasskeyCredential, RepositoryError>;
    async fn list_by_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<PasskeyCredential>, RepositoryError>;
    async fn list_all(&self) -> Result<Vec<PasskeyCredential>, RepositoryError>;
    async fn update_credential_json(
        &self,
        id: PasskeyCredentialId,
        credential_json: &str,
    ) -> Result<(), RepositoryError>;
    async fn update_last_used(&self, id: PasskeyCredentialId) -> Result<(), RepositoryError>;
    async fn delete(&self, id: PasskeyCredentialId) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait RegistrationTokenRepository: Send + Sync {
    async fn insert(
        &self,
        token: NewRegistrationToken,
    ) -> Result<RegistrationToken, RepositoryError>;
    async fn get_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<RegistrationToken, RepositoryError>;
    async fn mark_used(
        &self,
        id: RegistrationTokenId,
        user_id: UserId,
    ) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait AiUsageRepository: Send + Sync {
    async fn insert(&self, usage: NewAiUsage) -> Result<AiUsage, RepositoryError>;
    async fn summary_for_user(&self, user_id: UserId) -> Result<AiUsageSummary, RepositoryError>;
}

#[async_trait]
pub trait ImageRepository: Send + Sync {
    async fn upsert(&self, image: EntityImage) -> Result<(), RepositoryError>;
    async fn get(&self, entity_type: &str, entity_id: i64) -> Result<EntityImage, RepositoryError>;
    async fn get_thumbnail(
        &self,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<EntityImage, RepositoryError>;
    async fn delete(&self, entity_type: &str, entity_id: i64) -> Result<(), RepositoryError>;
    async fn has_image(&self, entity_type: &str, entity_id: i64) -> Result<bool, RepositoryError>;
    async fn entity_ids_with_images(
        &self,
        entity_type: &str,
        entity_ids: &[i64],
    ) -> Result<HashSet<i64>, RepositoryError>;
}

#[async_trait]
pub trait StatsRepository: Send + Sync {
    async fn book_summary(
        &self,
        user_id: UserId,
    ) -> Result<crate::domain::stats::BookSummaryStats, RepositoryError>;
    async fn reading_summary(
        &self,
        user_id: UserId,
    ) -> Result<crate::domain::stats::ReadingStats, RepositoryError>;
    async fn get_cached(
        &self,
        user_id: UserId,
    ) -> Result<Option<crate::domain::stats::CachedStats>, RepositoryError>;
    async fn store_cached(
        &self,
        user_id: UserId,
        stats: &crate::domain::stats::CachedStats,
    ) -> Result<(), RepositoryError>;
    /// Return distinct years (descending) that have at least one completed reading.
    async fn available_years(&self, user_id: UserId) -> Result<Vec<i32>, RepositoryError>;
    /// Book summary stats filtered to books with a completed reading in the given year.
    async fn book_summary_for_year(
        &self,
        user_id: UserId,
        year: i32,
    ) -> Result<crate::domain::stats::BookSummaryStats, RepositoryError>;
    /// Reading stats filtered to readings completed in the given year.
    async fn reading_summary_for_year(
        &self,
        user_id: UserId,
        year: i32,
    ) -> Result<crate::domain::stats::ReadingStats, RepositoryError>;
}

#[async_trait]
pub trait CoverSuggestionRepository: Send + Sync {
    async fn insert(&self, suggestion: CoverSuggestion) -> Result<(), RepositoryError>;
    async fn get(&self, id: &str) -> Result<CoverSuggestion, RepositoryError>;
    async fn get_thumbnail(&self, id: &str) -> Result<CoverSuggestion, RepositoryError>;
    async fn delete(&self, id: &str) -> Result<(), RepositoryError>;
    async fn delete_older_than(&self, max_age: Duration) -> Result<u64, RepositoryError>;
}
