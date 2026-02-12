use std::sync::Arc;

use tracing::warn;

use crate::domain::errors::RepositoryError;
use crate::domain::ids::ReadingId;
use crate::domain::readings::{NewReading, Reading, UpdateReading, reading_timeline_event};
use crate::domain::repositories::{
    AuthorRepository, BookRepository, ReadingRepository, TimelineEventRepository,
    UserBookRepository,
};
use crate::domain::user_books::{NewUserBook, Shelf};

#[derive(Clone)]
pub struct ReadingService {
    readings: Arc<dyn ReadingRepository>,
    books: Arc<dyn BookRepository>,
    authors: Arc<dyn AuthorRepository>,
    timeline: Arc<dyn TimelineEventRepository>,
    user_books: Arc<dyn UserBookRepository>,
}

impl ReadingService {
    pub fn new(
        readings: Arc<dyn ReadingRepository>,
        books: Arc<dyn BookRepository>,
        authors: Arc<dyn AuthorRepository>,
        timeline: Arc<dyn TimelineEventRepository>,
        user_books: Arc<dyn UserBookRepository>,
    ) -> Self {
        Self {
            readings,
            books,
            authors,
            timeline,
            user_books,
        }
    }

    pub async fn create(&self, new: NewReading) -> Result<Reading, RepositoryError> {
        let user_id = new.user_id;
        let book_id = new.book_id;
        let reading = self.readings.insert(new).await?;

        // Auto-add the book to the user's library when a reading is created,
        // or move it from wishlist to library if it's already there.
        match self.user_books.get_by_user_and_book(user_id, book_id).await {
            Ok(existing) if existing.shelf == Shelf::Wishlist => {
                if let Err(err) = self
                    .user_books
                    .move_shelf(existing.id, Shelf::Library)
                    .await
                {
                    warn!(error = %err, %user_id, %book_id, "failed to move book from wishlist to library");
                }
            }
            Err(_) => {
                let new_user_book = NewUserBook {
                    user_id,
                    book_id,
                    shelf: Shelf::default(),
                    book_club: false,
                };
                if let Err(err) = self.user_books.insert(new_user_book).await {
                    warn!(error = %err, %user_id, %book_id, "failed to auto-add book to library");
                }
            }
            Ok(_) => {} // Already in library, nothing to do
        }

        self.record_timeline_event(&reading).await;
        Ok(reading)
    }

    /// Transition a reading to finished status, records a timeline event.
    pub async fn finish(
        &self,
        id: ReadingId,
        mut update: UpdateReading,
    ) -> Result<Reading, RepositoryError> {
        if update.finished_at.is_none() {
            update.finished_at = Some(chrono::Utc::now().date_naive());
        }
        let reading = self.readings.update(id, update).await?;
        self.record_timeline_event(&reading).await;
        Ok(reading)
    }

    async fn record_timeline_event(&self, reading: &Reading) {
        let book = match self.books.get(reading.book_id).await {
            Ok(b) => b,
            Err(err) => {
                warn!(error = %err, reading_id = %reading.id, "failed to fetch book for reading timeline event");
                return;
            }
        };

        // Fetch authors via the book's author associations
        let mut authors = Vec::new();
        if let Ok(enriched) = self.books.get_with_authors(book.id).await {
            for author_info in &enriched.authors {
                if let Ok(author) = self.authors.get(author_info.author_id).await {
                    authors.push(author);
                }
            }
        }

        if let Err(err) = self
            .timeline
            .insert(reading_timeline_event(reading, &book, &authors))
            .await
        {
            warn!(error = %err, reading_id = %reading.id, "failed to record reading timeline event");
        }
    }
}
