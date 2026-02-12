use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::books::authors::Author;
use crate::domain::books::books::Book;
use crate::domain::ids::{BookId, ReadingId, UserBookId, UserId};
use crate::domain::listing::{SortDirection, SortKey};
use crate::domain::timeline::{NewTimelineEvent, TimelineEventDetail};

use super::books::BookWithAuthors;
use super::readings::ReadingStatus;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Shelf {
    #[default]
    Library,
    Wishlist,
}

impl Shelf {
    pub fn as_str(&self) -> &'static str {
        match self {
            Shelf::Library => "library",
            Shelf::Wishlist => "wishlist",
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            Shelf::Library => "Library",
            Shelf::Wishlist => "Wishlist",
        }
    }
}

impl FromStr for Shelf {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "library" => Ok(Shelf::Library),
            "wishlist" => Ok(Shelf::Wishlist),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBook {
    pub id: UserBookId,
    pub user_id: UserId,
    pub book_id: BookId,
    pub shelf: Shelf,
    pub book_club: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBookWithDetails {
    #[serde(flatten)]
    pub user_book: UserBook,
    pub book: BookWithAuthors,
    pub reading_summary: Option<ReadingSummary>,
}

/// Summary of the most recent reading for a user-book pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingSummary {
    pub reading_id: ReadingId,
    pub status: ReadingStatus,
    pub started_at: Option<NaiveDate>,
    pub finished_at: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUserBook {
    pub user_id: UserId,
    pub book_id: BookId,
    #[serde(default)]
    pub shelf: Shelf,
    #[serde(default)]
    pub book_club: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum UserBookSortKey {
    CreatedAt,
    Title,
    Author,
    Genre,
    Club,
    Status,
}

impl SortKey for UserBookSortKey {
    fn default() -> Self {
        UserBookSortKey::CreatedAt
    }

    fn from_query(value: &str) -> Option<Self> {
        match value {
            "created-at" => Some(UserBookSortKey::CreatedAt),
            "title" => Some(UserBookSortKey::Title),
            "author" => Some(UserBookSortKey::Author),
            "genre" => Some(UserBookSortKey::Genre),
            "club" => Some(UserBookSortKey::Club),
            "status" => Some(UserBookSortKey::Status),
            _ => None,
        }
    }

    fn query_value(self) -> &'static str {
        match self {
            UserBookSortKey::CreatedAt => "created-at",
            UserBookSortKey::Title => "title",
            UserBookSortKey::Author => "author",
            UserBookSortKey::Genre => "genre",
            UserBookSortKey::Club => "club",
            UserBookSortKey::Status => "status",
        }
    }

    fn default_direction(self) -> SortDirection {
        match self {
            UserBookSortKey::CreatedAt => SortDirection::Desc,
            _ => SortDirection::Asc,
        }
    }
}

pub fn user_book_timeline_event(
    user_book: &UserBook,
    book: &Book,
    authors: &[Author],
) -> NewTimelineEvent {
    let author_names: Vec<&str> = authors.iter().map(|a| a.name.as_str()).collect();
    let details = vec![TimelineEventDetail::author_detail(&author_names)];

    NewTimelineEvent {
        user_id: Some(user_book.user_id),
        entity_type: "book".to_string(),
        entity_id: book.id.into_inner(),
        action: "shelved".to_string(),
        occurred_at: user_book.created_at,
        title: book.title.clone(),
        details,
        genres: vec![],
        reading_data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shelf_from_str_valid() {
        assert_eq!("library".parse::<Shelf>(), Ok(Shelf::Library));
        assert_eq!("wishlist".parse::<Shelf>(), Ok(Shelf::Wishlist));
    }

    #[test]
    fn shelf_from_str_case_insensitive() {
        assert_eq!("LIBRARY".parse::<Shelf>(), Ok(Shelf::Library));
        assert_eq!("Wishlist".parse::<Shelf>(), Ok(Shelf::Wishlist));
    }

    #[test]
    fn shelf_from_str_invalid() {
        assert!("archive".parse::<Shelf>().is_err());
        assert!("".parse::<Shelf>().is_err());
    }

    #[test]
    fn shelf_roundtrip() {
        for shelf in [Shelf::Library, Shelf::Wishlist] {
            assert_eq!(shelf.as_str().parse::<Shelf>(), Ok(shelf));
        }
    }

    #[test]
    fn shelf_default_is_library() {
        assert_eq!(Shelf::default(), Shelf::Library);
    }
}
