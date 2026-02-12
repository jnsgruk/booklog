use std::sync::Arc;

use tracing::warn;

use crate::domain::book_items::{Book, NewBook, book_timeline_event};
use crate::domain::errors::RepositoryError;
use crate::domain::ids::UserId;
use crate::domain::repositories::{
    AuthorRepository, BookRepository, GenreRepository, TimelineEventRepository,
};

#[derive(Clone)]
pub struct BookService {
    books: Arc<dyn BookRepository>,
    authors: Arc<dyn AuthorRepository>,
    genres: Arc<dyn GenreRepository>,
    timeline: Arc<dyn TimelineEventRepository>,
}

impl BookService {
    pub fn new(
        books: Arc<dyn BookRepository>,
        authors: Arc<dyn AuthorRepository>,
        genres: Arc<dyn GenreRepository>,
        timeline: Arc<dyn TimelineEventRepository>,
    ) -> Self {
        Self {
            books,
            authors,
            genres,
            timeline,
        }
    }

    pub async fn create(&self, new: NewBook, user_id: UserId) -> Result<Book, RepositoryError> {
        let book = self.books.insert(new).await?;

        // Fetch authors for timeline enrichment
        let mut authors = Vec::new();
        if let Ok(enriched) = self.books.get_with_authors(book.id).await {
            for author_info in &enriched.authors {
                if let Ok(author) = self.authors.get(author_info.author_id).await {
                    authors.push(author);
                }
            }
        }

        // Fetch genre names for timeline enrichment
        let primary_genre = if let Some(id) = book.primary_genre_id {
            self.genres.get(id).await.ok().map(|g| g.name)
        } else {
            None
        };
        let secondary_genre = if let Some(id) = book.secondary_genre_id {
            self.genres.get(id).await.ok().map(|g| g.name)
        } else {
            None
        };

        if let Err(err) = self
            .timeline
            .insert(book_timeline_event(
                &book,
                &authors,
                primary_genre.as_deref(),
                secondary_genre.as_deref(),
                user_id,
            ))
            .await
        {
            warn!(error = %err, book_id = %book.id, "failed to record book timeline event");
        }

        Ok(book)
    }
}
