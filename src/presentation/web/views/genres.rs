use crate::domain::books::genres::Genre;

use super::genre_path;

pub struct GenreDetailView {
    pub id: String,
    pub name: String,
    pub created_date: String,
    pub created_time: String,
}

impl GenreDetailView {
    pub fn from_domain(genre: Genre) -> Self {
        Self {
            id: genre.id.to_string(),
            name: genre.name,
            created_date: genre.created_at.format("%Y-%m-%d").to_string(),
            created_time: genre.created_at.format("%H:%M").to_string(),
        }
    }
}

pub struct GenreOptionView {
    pub id: String,
    pub name: String,
}

impl From<Genre> for GenreOptionView {
    fn from(genre: Genre) -> Self {
        Self {
            id: genre.id.to_string(),
            name: genre.name,
        }
    }
}

impl From<&Genre> for GenreOptionView {
    fn from(genre: &Genre) -> Self {
        Self {
            id: genre.id.to_string(),
            name: genre.name.clone(),
        }
    }
}

pub struct GenreView {
    pub id: String,
    pub detail_path: String,
    pub name: String,
    pub created_date: String,
    pub created_time: String,
    pub created_at_sort_key: i64,
}

impl From<Genre> for GenreView {
    fn from(genre: Genre) -> Self {
        let Genre {
            id,
            name,
            created_at,
        } = genre;

        let detail_path = genre_path(id);

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
