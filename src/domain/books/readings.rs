use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::books::authors::Author;
use crate::domain::books::books::Book;
use crate::domain::ids::{BookId, ReadingId, UserId};
use crate::domain::listing::{SortDirection, SortKey};
use crate::domain::timeline::{NewTimelineEvent, TimelineEventDetail};

pub use super::quick_reviews::{QuickReview, Sentiment};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReadingStatus {
    #[default]
    Reading,
    Read,
    Abandoned,
}

impl ReadingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReadingStatus::Reading => "reading",
            ReadingStatus::Read => "read",
            ReadingStatus::Abandoned => "abandoned",
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            ReadingStatus::Reading => "Reading",
            ReadingStatus::Read => "Read",
            ReadingStatus::Abandoned => "Abandoned",
        }
    }
}

impl FromStr for ReadingStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "reading" => Ok(ReadingStatus::Reading),
            "read" => Ok(ReadingStatus::Read),
            "abandoned" => Ok(ReadingStatus::Abandoned),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingFormat {
    Physical,
    #[serde(rename = "ereader")]
    EReader,
    Audiobook,
}

impl ReadingFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReadingFormat::Physical => "physical",
            ReadingFormat::EReader => "ereader",
            ReadingFormat::Audiobook => "audiobook",
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            ReadingFormat::Physical => "Physical",
            ReadingFormat::EReader => "eReader",
            ReadingFormat::Audiobook => "Audiobook",
        }
    }
}

impl FromStr for ReadingFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "physical" => Ok(ReadingFormat::Physical),
            "ereader" | "e_reader" => Ok(ReadingFormat::EReader),
            "audiobook" => Ok(ReadingFormat::Audiobook),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reading {
    pub id: ReadingId,
    pub user_id: UserId,
    pub book_id: BookId,
    pub status: ReadingStatus,
    pub format: Option<ReadingFormat>,
    pub started_at: Option<NaiveDate>,
    pub finished_at: Option<NaiveDate>,
    pub rating: Option<f64>,
    pub quick_reviews: Vec<QuickReview>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingWithBook {
    #[serde(flatten)]
    pub reading: Reading,
    pub book_title: String,
    pub author_names: String,
    pub page_count: Option<i32>,
    pub year_published: Option<i32>,
    pub primary_genre: Option<String>,
    pub secondary_genre: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewReading {
    pub user_id: UserId,
    pub book_id: BookId,
    #[serde(default)]
    pub status: ReadingStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<ReadingFormat>,
    pub started_at: Option<NaiveDate>,
    pub finished_at: Option<NaiveDate>,
    pub rating: Option<f64>,
    #[serde(default)]
    pub quick_reviews: Vec<QuickReview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateReading {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub book_id: Option<BookId>,
    pub status: Option<ReadingStatus>,
    pub format: Option<ReadingFormat>,
    pub started_at: Option<NaiveDate>,
    pub finished_at: Option<NaiveDate>,
    pub rating: Option<f64>,
    pub quick_reviews: Option<Vec<QuickReview>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

/// Filter criteria for reading queries.
#[derive(Debug, Default, Clone)]
pub struct ReadingFilter {
    pub user_id: Option<UserId>,
    pub status: Option<ReadingStatus>,
    pub book_id: Option<BookId>,
}

impl ReadingFilter {
    pub fn all() -> Self {
        Self::default()
    }

    pub fn for_user(user_id: UserId) -> Self {
        Self {
            user_id: Some(user_id),
            ..Default::default()
        }
    }

    pub fn for_user_status(user_id: UserId, status: ReadingStatus) -> Self {
        Self {
            user_id: Some(user_id),
            status: Some(status),
            ..Default::default()
        }
    }

    pub fn for_status(status: ReadingStatus) -> Self {
        Self {
            status: Some(status),
            ..Default::default()
        }
    }

    pub fn for_book(book_id: BookId) -> Self {
        Self {
            book_id: Some(book_id),
            ..Default::default()
        }
    }

    pub fn for_user_book(user_id: UserId, book_id: BookId) -> Self {
        Self {
            user_id: Some(user_id),
            book_id: Some(book_id),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ReadingSortKey {
    CreatedAt,
    UpdatedAt,
    Status,
    Rating,
    StartedAt,
    FinishedAt,
    BookTitle,
}

impl SortKey for ReadingSortKey {
    fn default() -> Self {
        ReadingSortKey::CreatedAt
    }

    fn from_query(value: &str) -> Option<Self> {
        match value {
            "created-at" => Some(ReadingSortKey::CreatedAt),
            "updated-at" => Some(ReadingSortKey::UpdatedAt),
            "status" => Some(ReadingSortKey::Status),
            "rating" => Some(ReadingSortKey::Rating),
            "started-at" => Some(ReadingSortKey::StartedAt),
            "finished-at" => Some(ReadingSortKey::FinishedAt),
            "book-title" => Some(ReadingSortKey::BookTitle),
            _ => None,
        }
    }

    fn query_value(self) -> &'static str {
        match self {
            ReadingSortKey::CreatedAt => "created-at",
            ReadingSortKey::UpdatedAt => "updated-at",
            ReadingSortKey::Status => "status",
            ReadingSortKey::Rating => "rating",
            ReadingSortKey::StartedAt => "started-at",
            ReadingSortKey::FinishedAt => "finished-at",
            ReadingSortKey::BookTitle => "book-title",
        }
    }

    fn default_direction(self) -> SortDirection {
        match self {
            ReadingSortKey::BookTitle | ReadingSortKey::Status => SortDirection::Asc,
            _ => SortDirection::Desc,
        }
    }
}

pub fn reading_timeline_event(
    reading: &Reading,
    book: &Book,
    authors: &[Author],
) -> NewTimelineEvent {
    let action = match reading.status {
        ReadingStatus::Reading => "started",
        ReadingStatus::Read => "finished",
        ReadingStatus::Abandoned => "abandoned",
    };

    let author_names: Vec<&str> = authors.iter().map(|a| a.name.as_str()).collect();
    let mut details = vec![TimelineEventDetail::author_detail(&author_names)];

    if let Some(fmt) = reading.format {
        details.push(TimelineEventDetail {
            label: "Format".to_string(),
            value: fmt.display_label().to_string(),
        });
    }

    if let Some(rating) = reading.rating {
        details.push(TimelineEventDetail {
            label: "Rating".to_string(),
            value: crate::domain::formatting::format_rating(rating),
        });
    }

    if !reading.quick_reviews.is_empty() {
        let labels: Vec<&str> = reading.quick_reviews.iter().map(|r| r.label()).collect();
        details.push(TimelineEventDetail {
            label: "Notes".to_string(),
            value: labels.join(", "),
        });
    }

    let occurred_at = match reading.status {
        ReadingStatus::Read | ReadingStatus::Abandoned => reading.updated_at,
        ReadingStatus::Reading => reading.created_at,
    };

    NewTimelineEvent {
        user_id: Some(reading.user_id),
        entity_type: "reading".to_string(),
        entity_id: reading.id.into_inner(),
        action: action.to_string(),
        occurred_at,
        title: book.title.clone(),
        details,
        genres: vec![],
        reading_data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ReadingStatus ---

    #[test]
    fn reading_status_from_str_valid() {
        assert_eq!(
            "reading".parse::<ReadingStatus>(),
            Ok(ReadingStatus::Reading)
        );
        assert_eq!("read".parse::<ReadingStatus>(), Ok(ReadingStatus::Read));
        assert_eq!(
            "abandoned".parse::<ReadingStatus>(),
            Ok(ReadingStatus::Abandoned)
        );
    }

    #[test]
    fn reading_status_from_str_case_insensitive() {
        assert_eq!(
            "READING".parse::<ReadingStatus>(),
            Ok(ReadingStatus::Reading)
        );
        assert_eq!("Read".parse::<ReadingStatus>(), Ok(ReadingStatus::Read));
    }

    #[test]
    fn reading_status_from_str_invalid() {
        assert!("finished".parse::<ReadingStatus>().is_err());
        assert!("".parse::<ReadingStatus>().is_err());
    }

    #[test]
    fn reading_status_roundtrip() {
        for status in [
            ReadingStatus::Reading,
            ReadingStatus::Read,
            ReadingStatus::Abandoned,
        ] {
            assert_eq!(status.as_str().parse::<ReadingStatus>(), Ok(status));
        }
    }

    #[test]
    fn reading_status_default_is_reading() {
        assert_eq!(ReadingStatus::default(), ReadingStatus::Reading);
    }

    // --- ReadingFormat ---

    #[test]
    fn reading_format_from_str_valid() {
        assert_eq!(
            "physical".parse::<ReadingFormat>(),
            Ok(ReadingFormat::Physical)
        );
        assert_eq!(
            "ereader".parse::<ReadingFormat>(),
            Ok(ReadingFormat::EReader)
        );
        assert_eq!(
            "e_reader".parse::<ReadingFormat>(),
            Ok(ReadingFormat::EReader)
        );
        assert_eq!(
            "audiobook".parse::<ReadingFormat>(),
            Ok(ReadingFormat::Audiobook)
        );
    }

    #[test]
    fn reading_format_from_str_case_insensitive() {
        assert_eq!(
            "PHYSICAL".parse::<ReadingFormat>(),
            Ok(ReadingFormat::Physical)
        );
        assert_eq!(
            "EReader".parse::<ReadingFormat>(),
            Ok(ReadingFormat::EReader)
        );
    }

    #[test]
    fn reading_format_from_str_hyphenated() {
        assert_eq!(
            "e-reader".parse::<ReadingFormat>(),
            Ok(ReadingFormat::EReader)
        );
    }

    #[test]
    fn reading_format_from_str_invalid() {
        assert!("kindle".parse::<ReadingFormat>().is_err());
        assert!("".parse::<ReadingFormat>().is_err());
    }

    #[test]
    fn reading_format_roundtrip() {
        for fmt in [
            ReadingFormat::Physical,
            ReadingFormat::EReader,
            ReadingFormat::Audiobook,
        ] {
            assert_eq!(fmt.as_str().parse::<ReadingFormat>(), Ok(fmt));
        }
    }

    // --- QuickReview ---

    #[test]
    fn quick_review_all_returns_all_variants() {
        assert_eq!(QuickReview::all().len(), 19);
    }

    #[test]
    fn quick_review_form_value_roundtrip() {
        for review in QuickReview::all() {
            assert_eq!(
                QuickReview::from_str_value(review.form_value()),
                Some(*review),
                "form_value roundtrip failed for {:?}",
                review
            );
        }
    }

    #[test]
    fn quick_review_label_roundtrip() {
        for review in QuickReview::all() {
            assert_eq!(
                QuickReview::from_str_value(review.label()),
                Some(*review),
                "label roundtrip failed for {:?}",
                review
            );
        }
    }

    #[test]
    fn quick_review_from_str_value_invalid() {
        assert_eq!(QuickReview::from_str_value("nonexistent"), None);
        assert_eq!(QuickReview::from_str_value(""), None);
    }

    #[test]
    fn quick_review_sentiment_classification() {
        assert_eq!(QuickReview::LovedIt.sentiment(), Sentiment::Positive);
        assert_eq!(QuickReview::PageTurner.sentiment(), Sentiment::Positive);
        assert_eq!(QuickReview::QuickRead.sentiment(), Sentiment::Neutral);
        assert_eq!(QuickReview::SlowBurn.sentiment(), Sentiment::Neutral);
        assert_eq!(QuickReview::Dense.sentiment(), Sentiment::Neutral);
        assert_eq!(QuickReview::Forgettable.sentiment(), Sentiment::Negative);
        assert_eq!(QuickReview::Overrated.sentiment(), Sentiment::Negative);
    }

    #[test]
    fn quick_review_is_positive_matches_sentiment() {
        for review in QuickReview::all() {
            assert_eq!(
                review.is_positive(),
                review.sentiment() == Sentiment::Positive,
                "is_positive mismatch for {:?}",
                review
            );
        }
    }

    #[test]
    fn quick_review_is_neutral_matches_sentiment() {
        for review in QuickReview::all() {
            assert_eq!(
                review.is_neutral(),
                review.sentiment() == Sentiment::Neutral,
                "is_neutral mismatch for {:?}",
                review
            );
        }
    }

    #[test]
    fn quick_review_every_variant_has_sentiment() {
        for review in QuickReview::all() {
            let _ = review.sentiment(); // should not panic
        }
    }
}
