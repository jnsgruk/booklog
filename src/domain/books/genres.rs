use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{GenreId, UserId};
use crate::domain::listing::{SortDirection, SortKey};
use crate::domain::timeline::NewTimelineEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genre {
    pub id: GenreId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewGenre {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

impl NewGenre {
    pub fn normalize(mut self) -> Self {
        self.name = self.name.trim().to_string();
        self
    }
}

impl Genre {
    pub fn to_timeline_event(&self, user_id: UserId) -> NewTimelineEvent {
        NewTimelineEvent {
            user_id: Some(user_id),
            entity_type: "genre".to_string(),
            entity_id: self.id.into_inner(),
            action: "added".to_string(),
            occurred_at: self.created_at,
            title: self.name.clone(),
            details: vec![],
            genres: vec![],
            reading_data: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateGenre {
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GenreSortKey {
    CreatedAt,
    Name,
}

impl SortKey for GenreSortKey {
    fn default() -> Self {
        GenreSortKey::CreatedAt
    }

    fn from_query(value: &str) -> Option<Self> {
        match value {
            "created-at" => Some(GenreSortKey::CreatedAt),
            "name" => Some(GenreSortKey::Name),
            _ => None,
        }
    }

    fn query_value(self) -> &'static str {
        match self {
            GenreSortKey::CreatedAt => "created-at",
            GenreSortKey::Name => "name",
        }
    }

    fn default_direction(self) -> SortDirection {
        match self {
            GenreSortKey::CreatedAt => SortDirection::Desc,
            GenreSortKey::Name => SortDirection::Asc,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_trims_name() {
        let genre = NewGenre {
            name: "  Science Fiction  ".to_string(),
            created_at: None,
        }
        .normalize();
        assert_eq!(genre.name, "Science Fiction");
    }

    #[test]
    fn normalize_whitespace_only_name() {
        let genre = NewGenre {
            name: "   ".to_string(),
            created_at: None,
        }
        .normalize();
        assert_eq!(genre.name, "");
    }
}
