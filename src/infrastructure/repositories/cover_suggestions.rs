use async_trait::async_trait;
use sqlx::{query, query_as};

use crate::domain::RepositoryError;
use crate::domain::cover_suggestions::CoverSuggestion;
use crate::domain::repositories::CoverSuggestionRepository;
use crate::infrastructure::database::DatabasePool;

#[derive(Clone)]
pub struct SqlCoverSuggestionRepository {
    pool: DatabasePool,
}

impl SqlCoverSuggestionRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct CoverSuggestionRecord {
    id: String,
    image_data: Vec<u8>,
    thumbnail_data: Vec<u8>,
    content_type: String,
    source_url: String,
    created_at: String,
}

#[derive(sqlx::FromRow)]
struct ThumbnailRecord {
    id: String,
    thumbnail_data: Vec<u8>,
    content_type: String,
    source_url: String,
    created_at: String,
}

fn parse_timestamp(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ")
                .map(|naive| naive.and_utc())
        })
        .unwrap_or_default()
}

#[async_trait]
impl CoverSuggestionRepository for SqlCoverSuggestionRepository {
    async fn insert(&self, suggestion: CoverSuggestion) -> Result<(), RepositoryError> {
        query(
            r"INSERT INTO cover_suggestions (id, image_data, thumbnail_data, content_type, source_url)
               VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&suggestion.id)
        .bind(&suggestion.image_data)
        .bind(&suggestion.thumbnail_data)
        .bind(&suggestion.content_type)
        .bind(&suggestion.source_url)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::unexpected(e.to_string()))?;

        Ok(())
    }

    async fn get(&self, id: &str) -> Result<CoverSuggestion, RepositoryError> {
        let record = query_as::<_, CoverSuggestionRecord>(
            r"SELECT id, image_data, thumbnail_data, content_type, source_url, created_at
               FROM cover_suggestions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::unexpected(e.to_string()))?
        .ok_or(RepositoryError::NotFound)?;

        Ok(CoverSuggestion {
            id: record.id,
            image_data: record.image_data,
            thumbnail_data: record.thumbnail_data,
            content_type: record.content_type,
            source_url: record.source_url,
            created_at: parse_timestamp(&record.created_at),
        })
    }

    async fn get_thumbnail(&self, id: &str) -> Result<CoverSuggestion, RepositoryError> {
        let record = query_as::<_, ThumbnailRecord>(
            r"SELECT id, thumbnail_data, content_type, source_url, created_at
               FROM cover_suggestions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::unexpected(e.to_string()))?
        .ok_or(RepositoryError::NotFound)?;

        Ok(CoverSuggestion {
            id: record.id,
            image_data: Vec::new(),
            thumbnail_data: record.thumbnail_data,
            content_type: record.content_type,
            source_url: record.source_url,
            created_at: parse_timestamp(&record.created_at),
        })
    }

    async fn delete(&self, id: &str) -> Result<(), RepositoryError> {
        query(r"DELETE FROM cover_suggestions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::unexpected(e.to_string()))?;

        Ok(())
    }

    async fn delete_older_than(
        &self,
        max_age: std::time::Duration,
    ) -> Result<u64, RepositoryError> {
        let cutoff_secs = i64::try_from(max_age.as_secs())
            .map_err(|_| RepositoryError::unexpected("max_age duration too large".to_string()))?;
        let result = query(
            r"DELETE FROM cover_suggestions
               WHERE created_at < strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-' || ? || ' seconds')",
        )
        .bind(cutoff_secs)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::unexpected(e.to_string()))?;

        Ok(result.rows_affected())
    }
}
