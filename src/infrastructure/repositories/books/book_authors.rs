use std::str::FromStr;

use sqlx::QueryBuilder;

use crate::domain::RepositoryError;
use crate::domain::book_items::{AuthorRole, BookAuthorInfo};
use crate::domain::ids::AuthorId;
use crate::infrastructure::database::DatabasePool;

#[derive(sqlx::FromRow)]
pub(crate) struct BookAuthorRecord {
    pub book_id: i64,
    pub author_id: i64,
    pub author_name: String,
    pub role: String,
}

impl BookAuthorRecord {
    pub fn to_info(&self) -> BookAuthorInfo {
        BookAuthorInfo {
            author_id: AuthorId::from(self.author_id),
            author_name: self.author_name.clone(),
            role: AuthorRole::from_str(&self.role).unwrap_or_else(|()| {
                tracing::warn!(role = %self.role, "unknown author role, defaulting to Author");
                AuthorRole::default()
            }),
        }
    }
}

pub(crate) async fn fetch_authors_for_books(
    pool: &DatabasePool,
    book_ids: &[i64],
) -> Result<Vec<BookAuthorRecord>, RepositoryError> {
    if book_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut qb = QueryBuilder::new(
        r"SELECT ba.book_id, ba.author_id, a.name AS author_name, ba.role
          FROM book_authors ba
          JOIN authors a ON a.id = ba.author_id
          WHERE ba.book_id IN (",
    );

    let mut sep = qb.separated(", ");
    for id in book_ids {
        sep.push_bind(*id);
    }
    sep.push_unseparated(") ORDER BY ba.rowid");

    qb.build_query_as::<BookAuthorRecord>()
        .fetch_all(pool)
        .await
        .map_err(|err| RepositoryError::unexpected(err.to_string()))
}
