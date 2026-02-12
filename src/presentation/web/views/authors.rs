use crate::domain::books::authors::Author;

use super::author_path;

pub struct AuthorDetailView {
    pub id: String,
    pub name: String,
    pub created_date: String,
    pub created_time: String,
}

impl AuthorDetailView {
    pub fn from_domain(author: Author) -> Self {
        Self {
            id: author.id.to_string(),
            name: author.name,
            created_date: author.created_at.format("%Y-%m-%d").to_string(),
            created_time: author.created_at.format("%H:%M").to_string(),
        }
    }
}

pub struct AuthorOptionView {
    pub id: String,
    pub name: String,
}

impl From<Author> for AuthorOptionView {
    fn from(author: Author) -> Self {
        Self {
            id: author.id.to_string(),
            name: author.name,
        }
    }
}

impl From<&Author> for AuthorOptionView {
    fn from(author: &Author) -> Self {
        Self {
            id: author.id.to_string(),
            name: author.name.clone(),
        }
    }
}

pub struct AuthorView {
    pub id: String,
    pub detail_path: String,
    pub name: String,
    pub created_date: String,
    pub created_time: String,
    pub created_at_sort_key: i64,
}

impl From<Author> for AuthorView {
    fn from(author: Author) -> Self {
        let Author {
            id,
            name,
            created_at,
        } = author;

        let detail_path = author_path(id);

        let created_at_sort_key = created_at.timestamp();
        let created_date = created_at.format("%Y-%m-%d").to_string();
        let created_time = created_at.format("%H:%M").to_string();

        Self {
            detail_path,
            id: id.to_string(),
            name,
            created_date,
            created_time,
            created_at_sort_key,
        }
    }
}
