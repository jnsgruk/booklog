use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{TimelineEventId, UserId};
use crate::domain::listing::{SortDirection, SortKey};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEventDetail {
    pub label: String,
    pub value: String,
}

impl TimelineEventDetail {
    pub fn author_detail(author_names: &[&str]) -> Self {
        Self {
            label: "Author".to_string(),
            value: if author_names.is_empty() {
                "Unknown".to_string()
            } else {
                author_names.join(", ")
            },
        }
    }
}

/// Reading data attached to timeline events for quick reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineReadingData {
    pub book_id: i64,
    pub rating: Option<i32>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: TimelineEventId,
    pub entity_type: String,
    pub entity_id: i64,
    pub action: String,
    pub occurred_at: DateTime<Utc>,
    pub title: String,
    pub details: Vec<TimelineEventDetail>,
    pub genres: Vec<String>,
    pub reading_data: Option<TimelineReadingData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTimelineEvent {
    pub user_id: Option<UserId>,
    pub entity_type: String,
    pub entity_id: i64,
    pub action: String,
    pub occurred_at: DateTime<Utc>,
    pub title: String,
    pub details: Vec<TimelineEventDetail>,
    pub genres: Vec<String>,
    pub reading_data: Option<TimelineReadingData>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TimelineSortKey {
    OccurredAt,
}

impl SortKey for TimelineSortKey {
    fn default() -> Self {
        TimelineSortKey::OccurredAt
    }

    fn from_query(value: &str) -> Option<Self> {
        match value {
            "occurred-at" => Some(TimelineSortKey::OccurredAt),
            _ => None,
        }
    }

    fn query_value(self) -> &'static str {
        match self {
            TimelineSortKey::OccurredAt => "occurred-at",
        }
    }

    fn default_direction(self) -> SortDirection {
        match self {
            TimelineSortKey::OccurredAt => SortDirection::Desc,
        }
    }
}
