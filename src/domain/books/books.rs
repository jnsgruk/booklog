use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::books::authors::Author;
use crate::domain::ids::{AuthorId, BookId, GenreId, UserId};
use crate::domain::listing::{SortDirection, SortKey};
use crate::domain::timeline::{NewTimelineEvent, TimelineEventDetail};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub id: BookId,
    pub title: String,
    pub isbn: Option<String>,
    pub description: Option<String>,
    pub page_count: Option<i32>,
    pub year_published: Option<i32>,
    pub publisher: Option<String>,
    pub language: Option<String>,
    pub primary_genre_id: Option<GenreId>,
    pub secondary_genre_id: Option<GenreId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookWithAuthors {
    #[serde(flatten)]
    pub book: Book,
    pub authors: Vec<BookAuthorInfo>,
    pub primary_genre: Option<String>,
    pub secondary_genre: Option<String>,
}

impl BookWithAuthors {
    /// Collect genre names into a vec (for timeline events, stats, etc.).
    pub fn genre_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        if let Some(ref g) = self.primary_genre {
            names.push(g.clone());
        }
        if let Some(ref g) = self.secondary_genre {
            names.push(g.clone());
        }
        names
    }
}

/// Lightweight author info embedded in book listings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookAuthorInfo {
    pub author_id: AuthorId,
    pub author_name: String,
    pub role: AuthorRole,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AuthorRole {
    #[default]
    Author,
    Editor,
    Translator,
}

impl AuthorRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthorRole::Author => "author",
            AuthorRole::Editor => "editor",
            AuthorRole::Translator => "translator",
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            AuthorRole::Author => "Author",
            AuthorRole::Editor => "Editor",
            AuthorRole::Translator => "Translator",
        }
    }
}

impl FromStr for AuthorRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "author" => Ok(AuthorRole::Author),
            "editor" => Ok(AuthorRole::Editor),
            "translator" => Ok(AuthorRole::Translator),
            _ => Err(()),
        }
    }
}

/// A book-author association for insert/update operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookAuthor {
    pub author_id: AuthorId,
    #[serde(default)]
    pub role: AuthorRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewBook {
    pub title: String,
    pub isbn: Option<String>,
    pub description: Option<String>,
    pub page_count: Option<i32>,
    pub year_published: Option<i32>,
    pub publisher: Option<String>,
    pub language: Option<String>,
    pub primary_genre_id: Option<GenreId>,
    pub secondary_genre_id: Option<GenreId>,
    pub authors: Vec<BookAuthor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

impl NewBook {
    pub fn normalize(mut self) -> Self {
        self.title = self.title.trim().to_string();
        self.isbn = normalize_optional_field(self.isbn);
        self.description = normalize_optional_field(self.description);
        self.publisher = normalize_optional_field(self.publisher);
        self.language = normalize_optional_field(self.language);
        self.page_count = self.page_count.filter(|&p| p > 0);
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateBook {
    pub title: Option<String>,
    pub isbn: Option<String>,
    pub description: Option<String>,
    pub page_count: Option<i32>,
    pub year_published: Option<i32>,
    pub publisher: Option<String>,
    pub language: Option<String>,
    /// `None` = don't change, `Some(None)` = clear, `Some(Some(id))` = set.
    pub primary_genre_id: Option<Option<GenreId>>,
    /// `None` = don't change, `Some(None)` = clear, `Some(Some(id))` = set.
    pub secondary_genre_id: Option<Option<GenreId>>,
    pub authors: Option<Vec<BookAuthor>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

impl UpdateBook {
    pub fn normalize(mut self) -> Self {
        self.isbn = normalize_optional_field(self.isbn);
        self.description = normalize_optional_field(self.description);
        self.publisher = normalize_optional_field(self.publisher);
        self.language = normalize_optional_field(self.language);
        self.page_count = self.page_count.filter(|&p| p > 0);
        self
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BookSortKey {
    CreatedAt,
    Title,
    Author,
    YearPublished,
    Publisher,
}

impl SortKey for BookSortKey {
    fn default() -> Self {
        BookSortKey::CreatedAt
    }

    fn from_query(value: &str) -> Option<Self> {
        match value {
            "created-at" => Some(BookSortKey::CreatedAt),
            "title" => Some(BookSortKey::Title),
            "author" => Some(BookSortKey::Author),
            "year-published" => Some(BookSortKey::YearPublished),
            "publisher" => Some(BookSortKey::Publisher),
            _ => None,
        }
    }

    fn query_value(self) -> &'static str {
        match self {
            BookSortKey::CreatedAt => "created-at",
            BookSortKey::Title => "title",
            BookSortKey::Author => "author",
            BookSortKey::YearPublished => "year-published",
            BookSortKey::Publisher => "publisher",
        }
    }

    fn default_direction(self) -> SortDirection {
        match self {
            BookSortKey::CreatedAt | BookSortKey::YearPublished => SortDirection::Desc,
            _ => SortDirection::Asc,
        }
    }
}

fn normalize_optional_field(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub fn book_timeline_event(
    book: &Book,
    authors: &[Author],
    primary_genre: Option<&str>,
    secondary_genre: Option<&str>,
    user_id: UserId,
) -> NewTimelineEvent {
    let author_names: Vec<&str> = authors.iter().map(|a| a.name.as_str()).collect();
    let mut details = vec![TimelineEventDetail::author_detail(&author_names)];
    let genres: Vec<String> = [primary_genre, secondary_genre]
        .iter()
        .filter_map(|g| g.map(String::from))
        .collect();
    if !genres.is_empty() {
        details.push(TimelineEventDetail {
            label: "Genres".to_string(),
            value: genres.join(", "),
        });
    }
    if let Some(pages) = book.page_count {
        details.push(TimelineEventDetail {
            label: "Pages".to_string(),
            value: format!("{pages}"),
        });
    }
    NewTimelineEvent {
        user_id: Some(user_id),
        entity_type: "book".to_string(),
        entity_id: book.id.into_inner(),
        action: "added".to_string(),
        occurred_at: book.created_at,
        title: book.title.clone(),
        details,
        genres,
        reading_data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- AuthorRole ---

    #[test]
    fn author_role_from_str_valid() {
        assert_eq!("author".parse::<AuthorRole>(), Ok(AuthorRole::Author));
        assert_eq!("editor".parse::<AuthorRole>(), Ok(AuthorRole::Editor));
        assert_eq!(
            "translator".parse::<AuthorRole>(),
            Ok(AuthorRole::Translator)
        );
    }

    #[test]
    fn author_role_from_str_case_insensitive() {
        assert_eq!("AUTHOR".parse::<AuthorRole>(), Ok(AuthorRole::Author));
        assert_eq!("Editor".parse::<AuthorRole>(), Ok(AuthorRole::Editor));
    }

    #[test]
    fn author_role_from_str_invalid() {
        assert!("writer".parse::<AuthorRole>().is_err());
        assert!("".parse::<AuthorRole>().is_err());
    }

    #[test]
    fn author_role_roundtrip() {
        for role in [
            AuthorRole::Author,
            AuthorRole::Editor,
            AuthorRole::Translator,
        ] {
            assert_eq!(role.as_str().parse::<AuthorRole>(), Ok(role));
        }
    }

    // --- NewBook normalization ---

    #[test]
    fn normalize_trims_title() {
        let book = NewBook {
            title: "  Hello World  ".to_string(),
            isbn: None,
            description: None,
            page_count: None,
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            authors: vec![],
            created_at: None,
        }
        .normalize();
        assert_eq!(book.title, "Hello World");
    }

    #[test]
    fn normalize_empty_optional_to_none() {
        let book = NewBook {
            title: "Test".to_string(),
            isbn: Some("  ".to_string()),
            description: Some("".to_string()),
            page_count: None,
            year_published: None,
            publisher: Some("   ".to_string()),
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            authors: vec![],
            created_at: None,
        }
        .normalize();
        assert_eq!(book.isbn, None);
        assert_eq!(book.description, None);
        assert_eq!(book.publisher, None);
    }

    #[test]
    fn normalize_trims_optional_fields() {
        let book = NewBook {
            title: "Test".to_string(),
            isbn: Some("  978-0-13-468599-1  ".to_string()),
            description: None,
            page_count: None,
            year_published: None,
            publisher: Some("  O'Reilly  ".to_string()),
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            authors: vec![],
            created_at: None,
        }
        .normalize();
        assert_eq!(book.isbn, Some("978-0-13-468599-1".to_string()));
        assert_eq!(book.publisher, Some("O'Reilly".to_string()));
    }

    #[test]
    fn normalize_zero_page_count_to_none() {
        let book = NewBook {
            title: "Test".to_string(),
            isbn: None,
            description: None,
            page_count: Some(0),
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            authors: vec![],
            created_at: None,
        }
        .normalize();
        assert_eq!(book.page_count, None);
    }

    #[test]
    fn normalize_negative_page_count_to_none() {
        let book = NewBook {
            title: "Test".to_string(),
            isbn: None,
            description: None,
            page_count: Some(-5),
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            authors: vec![],
            created_at: None,
        }
        .normalize();
        assert_eq!(book.page_count, None);
    }

    #[test]
    fn normalize_positive_page_count_kept() {
        let book = NewBook {
            title: "Test".to_string(),
            isbn: None,
            description: None,
            page_count: Some(300),
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            authors: vec![],
            created_at: None,
        }
        .normalize();
        assert_eq!(book.page_count, Some(300));
    }
}
