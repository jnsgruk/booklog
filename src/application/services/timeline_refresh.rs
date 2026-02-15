use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::domain::book_items::book_timeline_event;
use crate::domain::ids::UserId;
use crate::domain::listing::{ListRequest, SortKey};
use crate::domain::readings::{ReadingFilter, ReadingSortKey, reading_timeline_event};
use crate::domain::repositories::{
    AuthorRepository, BookRepository, GenreRepository, ReadingRepository, TimelineEventRepository,
};

/// Describes what needs to be refreshed.
#[derive(Debug, Clone)]
pub enum TimelineInvalidation {
    /// A specific entity was updated — refresh its timeline events and cascading dependents.
    SpecificEntity { entity_type: String, entity_id: i64 },
    /// Full rebuild — refresh all timeline events from scratch.
    Full,
}

/// Sends invalidation signals to the background timeline refresh task.
/// Non-blocking and fire-and-forget — safe to call from any handler.
#[derive(Clone)]
pub struct TimelineInvalidator {
    tx: mpsc::Sender<TimelineInvalidation>,
}

impl TimelineInvalidator {
    pub fn new(tx: mpsc::Sender<TimelineInvalidation>) -> Self {
        Self { tx }
    }

    /// Signal that a specific entity's timeline events need refreshing.
    pub fn invalidate(&self, entity_type: &str, entity_id: i64) {
        let _ = self.tx.try_send(TimelineInvalidation::SpecificEntity {
            entity_type: entity_type.to_string(),
            entity_id,
        });
    }

    /// Signal a full timeline rebuild.
    pub fn invalidate_full(&self) {
        let _ = self.tx.try_send(TimelineInvalidation::Full);
    }
}

/// Holds all repositories needed to rebuild timeline event snapshots.
#[allow(clippy::struct_field_names)]
struct TimelineRebuilder {
    author_repo: Arc<dyn AuthorRepository>,
    book_repo: Arc<dyn BookRepository>,
    genre_repo: Arc<dyn GenreRepository>,
    reading_repo: Arc<dyn ReadingRepository>,
    timeline_repo: Arc<dyn TimelineEventRepository>,
}

impl TimelineRebuilder {
    /// Refresh timeline events for a specific entity and its cascading dependents.
    async fn refresh_entity(&self, entity_type: &str, entity_id: i64) {
        match entity_type {
            "author" => self.refresh_author_cascade(entity_id).await,
            "book" => self.refresh_book_cascade(entity_id).await,
            "reading" => self.refresh_reading(entity_id).await,
            "genre" => self.refresh_genre_cascade(entity_id).await,
            other => warn!(
                entity_type = other,
                "unknown entity type for timeline refresh"
            ),
        }
    }

    /// Refresh an author's timeline event, then cascade to all books and their readings.
    async fn refresh_author_cascade(&self, author_id: i64) {
        let author_id_typed = crate::domain::ids::AuthorId::new(author_id);

        // Refresh the author's own event
        if let Ok(author) = self.author_repo.get(author_id_typed).await {
            // Author events have: title=name, no details, no genres
            if let Err(err) = self
                .timeline_repo
                .update_by_entity("author", author_id, &author.name, &[], &[], None)
                .await
            {
                warn!(error = %err, %author_id, "failed to refresh author timeline event");
            }
        }

        // Cascade: refresh all books by this author (they include author names in details)
        if let Ok(books) = self.book_repo.list_by_author(author_id_typed).await {
            for bwa in &books {
                self.refresh_book_event(bwa.book.id.into_inner()).await;
                // Cascade further: refresh readings of these books
                self.refresh_readings_for_book(bwa.book.id.into_inner())
                    .await;
            }
        }
    }

    /// Refresh a book's timeline event, then cascade to all its readings.
    async fn refresh_book_cascade(&self, book_id: i64) {
        self.refresh_book_event(book_id).await;
        self.refresh_readings_for_book(book_id).await;
    }

    /// Refresh a single book's timeline event snapshot.
    async fn refresh_book_event(&self, book_id: i64) {
        let book_id_typed = crate::domain::ids::BookId::new(book_id);

        let enriched = match self.book_repo.get_with_authors(book_id_typed).await {
            Ok(e) => e,
            Err(err) => {
                warn!(error = %err, %book_id, "failed to fetch book for timeline refresh");
                return;
            }
        };

        // Fetch full author objects for names
        let mut authors = Vec::new();
        for ba in &enriched.authors {
            if let Ok(author) = self.author_repo.get(ba.author_id).await {
                authors.push(author);
            }
        }

        // Fetch genre names
        let primary_genre = if let Some(gid) = enriched.book.primary_genre_id {
            self.genre_repo.get(gid).await.ok().map(|g| g.name)
        } else {
            None
        };
        let secondary_genre = if let Some(gid) = enriched.book.secondary_genre_id {
            self.genre_repo.get(gid).await.ok().map(|g| g.name)
        } else {
            None
        };

        // Use a dummy user_id — we only extract title/details/genres from the event
        let dummy_user_id = UserId::new(0);
        let event = book_timeline_event(
            &enriched.book,
            &authors,
            primary_genre.as_deref(),
            secondary_genre.as_deref(),
            dummy_user_id,
        );

        if let Err(err) = self
            .timeline_repo
            .update_by_entity(
                "book",
                book_id,
                &event.title,
                &event.details,
                &event.genres,
                None,
            )
            .await
        {
            warn!(error = %err, %book_id, "failed to refresh book timeline event");
        }
    }

    /// Refresh all reading timeline events for a given book.
    async fn refresh_readings_for_book(&self, book_id: i64) {
        let book_id_typed = crate::domain::ids::BookId::new(book_id);
        let filter = ReadingFilter::for_book(book_id_typed);
        let sort_key = ReadingSortKey::default();
        let request = ListRequest::show_all(sort_key, sort_key.default_direction());

        if let Ok(page) = self.reading_repo.list(filter, &request, None).await {
            for rwb in &page.items {
                self.refresh_reading(rwb.reading.id.into_inner()).await;
            }
        }
    }

    /// Refresh a single reading's timeline event snapshot.
    async fn refresh_reading(&self, reading_id: i64) {
        let reading_id_typed = crate::domain::ids::ReadingId::new(reading_id);

        let reading = match self.reading_repo.get(reading_id_typed).await {
            Ok(r) => r,
            Err(err) => {
                warn!(error = %err, %reading_id, "failed to fetch reading for timeline refresh");
                return;
            }
        };

        let book = match self.book_repo.get(reading.book_id).await {
            Ok(b) => b,
            Err(err) => {
                warn!(error = %err, %reading_id, "failed to fetch book for reading timeline refresh");
                return;
            }
        };

        // Fetch authors
        let mut authors = Vec::new();
        if let Ok(enriched) = self.book_repo.get_with_authors(book.id).await {
            for ba in &enriched.authors {
                if let Ok(author) = self.author_repo.get(ba.author_id).await {
                    authors.push(author);
                }
            }
        }

        let event = reading_timeline_event(&reading, &book, &authors);

        if let Err(err) = self
            .timeline_repo
            .update_by_entity(
                "reading",
                reading_id,
                &event.title,
                &event.details,
                &event.genres,
                event.reading_data.as_ref(),
            )
            .await
        {
            warn!(error = %err, %reading_id, "failed to refresh reading timeline event");
        }
    }

    /// Refresh a genre's timeline event, then cascade to all books with that genre.
    async fn refresh_genre_cascade(&self, genre_id: i64) {
        let genre_id_typed = crate::domain::ids::GenreId::new(genre_id);

        // Refresh the genre's own event
        if let Ok(genre) = self.genre_repo.get(genre_id_typed).await
            && let Err(err) = self
                .timeline_repo
                .update_by_entity("genre", genre_id, &genre.name, &[], &[], None)
                .await
        {
            warn!(error = %err, %genre_id, "failed to refresh genre timeline event");
        }

        // Cascade: refresh all books that use this genre
        if let Ok(books) = self.book_repo.list_by_genre(genre_id_typed).await {
            for bwa in &books {
                self.refresh_book_event(bwa.book.id.into_inner()).await;
            }
        }
    }

    /// Full rebuild: refresh every entity's timeline events.
    async fn full_rebuild(&self) {
        info!("starting full timeline rebuild");

        // Refresh all authors
        if let Ok(authors) = self.author_repo.list_all().await {
            for author in &authors {
                if let Err(err) = self
                    .timeline_repo
                    .update_by_entity(
                        "author",
                        author.id.into_inner(),
                        &author.name,
                        &[],
                        &[],
                        None,
                    )
                    .await
                {
                    warn!(error = %err, author_id = %author.id, "failed to refresh author during rebuild");
                }
            }
        }

        // Refresh all genres
        if let Ok(genres) = self.genre_repo.list_all().await {
            for genre in &genres {
                if let Err(err) = self
                    .timeline_repo
                    .update_by_entity("genre", genre.id.into_inner(), &genre.name, &[], &[], None)
                    .await
                {
                    warn!(error = %err, genre_id = %genre.id, "failed to refresh genre during rebuild");
                }
            }
        }

        // Refresh all books (includes author/genre enrichment)
        if let Ok(books) = self.book_repo.list_all().await {
            for bwa in &books {
                self.refresh_book_event(bwa.book.id.into_inner()).await;
            }
        }

        // Refresh all readings
        let sort_key = ReadingSortKey::default();
        let request = ListRequest::show_all(sort_key, sort_key.default_direction());
        if let Ok(page) = self
            .reading_repo
            .list(ReadingFilter::all(), &request, None)
            .await
        {
            for rwb in &page.items {
                self.refresh_reading(rwb.reading.id.into_inner()).await;
            }
        }

        info!("full timeline rebuild complete");
    }
}

/// Background task that listens for invalidation signals, debounces, and refreshes.
/// Mirrors the `stats_recomputation_task` pattern.
pub async fn timeline_rebuild_task(
    mut rx: mpsc::Receiver<TimelineInvalidation>,
    author_repo: Arc<dyn AuthorRepository>,
    book_repo: Arc<dyn BookRepository>,
    genre_repo: Arc<dyn GenreRepository>,
    reading_repo: Arc<dyn ReadingRepository>,
    timeline_repo: Arc<dyn TimelineEventRepository>,
    debounce: Duration,
) {
    let rebuilder = TimelineRebuilder {
        author_repo,
        book_repo,
        genre_repo,
        reading_repo,
        timeline_repo,
    };

    loop {
        let Some(first) = rx.recv().await else {
            break;
        };

        // Debounce: wait then drain any accumulated signals
        let mut needs_full = matches!(first, TimelineInvalidation::Full);
        let mut entities: HashSet<(String, i64)> = HashSet::new();

        if let TimelineInvalidation::SpecificEntity {
            entity_type,
            entity_id,
        } = first
        {
            entities.insert((entity_type, entity_id));
        }

        tokio::time::sleep(debounce).await;
        while let Ok(signal) = rx.try_recv() {
            match signal {
                TimelineInvalidation::Full => needs_full = true,
                TimelineInvalidation::SpecificEntity {
                    entity_type,
                    entity_id,
                } => {
                    entities.insert((entity_type, entity_id));
                }
            }
        }

        if needs_full {
            rebuilder.full_rebuild().await;
        } else {
            for (entity_type, entity_id) in &entities {
                rebuilder.refresh_entity(entity_type, *entity_id).await;
            }
        }
    }
}
