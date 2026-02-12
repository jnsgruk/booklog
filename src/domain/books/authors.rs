use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{AuthorId, UserId};
use crate::domain::listing::{SortDirection, SortKey};
use crate::domain::timeline::NewTimelineEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub id: AuthorId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAuthor {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

impl NewAuthor {
    pub fn normalize(mut self) -> Self {
        self.name = self.name.trim().to_string();
        self
    }
}

impl Author {
    pub fn to_timeline_event(&self, user_id: UserId) -> NewTimelineEvent {
        NewTimelineEvent {
            user_id: Some(user_id),
            entity_type: "author".to_string(),
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
pub struct UpdateAuthor {
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AuthorSortKey {
    CreatedAt,
    Name,
}

impl SortKey for AuthorSortKey {
    fn default() -> Self {
        AuthorSortKey::CreatedAt
    }

    fn from_query(value: &str) -> Option<Self> {
        match value {
            "created-at" => Some(AuthorSortKey::CreatedAt),
            "name" => Some(AuthorSortKey::Name),
            _ => None,
        }
    }

    fn query_value(self) -> &'static str {
        match self {
            AuthorSortKey::CreatedAt => "created-at",
            AuthorSortKey::Name => "name",
        }
    }

    fn default_direction(self) -> SortDirection {
        match self {
            AuthorSortKey::CreatedAt => SortDirection::Desc,
            AuthorSortKey::Name => SortDirection::Asc,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_trims_name() {
        let author = NewAuthor {
            name: "  J.R.R. Tolkien  ".to_string(),
            created_at: None,
        }
        .normalize();
        assert_eq!(author.name, "J.R.R. Tolkien");
    }

    #[test]
    fn normalize_whitespace_only_name() {
        let author = NewAuthor {
            name: "   ".to_string(),
            created_at: None,
        }
        .normalize();
        assert_eq!(author.name, "");
    }
}
