use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{QueryBuilder, query_as};

use crate::domain::RepositoryError;
use crate::domain::ids::{BookId, ReadingId, UserId};
use crate::domain::listing::{ListRequest, Page};
use crate::domain::readings::{
    NewReading, QuickReview, Reading, ReadingFilter, ReadingFormat, ReadingSortKey, ReadingStatus,
    ReadingWithBook, UpdateReading,
};
use crate::domain::repositories::ReadingRepository;
use crate::infrastructure::database::DatabasePool;
use crate::infrastructure::repositories::macros::push_update_field;

const BASE_SELECT: &str = r"
    SELECT
        r.id, r.user_id, r.book_id, r.status, r.format, r.started_at, r.finished_at, r.rating, r.review, r.created_at, r.updated_at,
        bk.title AS book_title,
        bk.page_count, bk.year_published,
        pg.name AS primary_genre, sg.name AS secondary_genre,
        COALESCE(GROUP_CONCAT(a.name, ', '), '') AS author_names
    FROM readings r
    JOIN books bk ON r.book_id = bk.id
    LEFT JOIN genres pg ON pg.id = bk.primary_genre_id
    LEFT JOIN genres sg ON sg.id = bk.secondary_genre_id
    LEFT JOIN book_authors ba ON ba.book_id = bk.id
    LEFT JOIN authors a ON a.id = ba.author_id
";

const BASE_GROUP_BY: &str = " GROUP BY r.id";

#[derive(Clone)]
pub struct SqlReadingRepository {
    pool: DatabasePool,
}

impl SqlReadingRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    fn order_clause(request: &ListRequest<ReadingSortKey>) -> String {
        let dir_sql = request.sort_direction().as_sql();

        match request.sort_key() {
            ReadingSortKey::CreatedAt => format!("r.created_at {dir_sql}, r.id DESC"),
            ReadingSortKey::UpdatedAt => format!("r.updated_at {dir_sql}, r.id DESC"),
            ReadingSortKey::Status => format!(
                "CASE r.status WHEN 'reading' THEN 1 WHEN 'read' THEN 2 WHEN 'abandoned' THEN 3 END {dir_sql}, r.created_at DESC"
            ),
            ReadingSortKey::Rating => {
                format!("COALESCE(r.rating, 0) {dir_sql}, r.created_at DESC")
            }
            ReadingSortKey::StartedAt => {
                format!("r.started_at {dir_sql}, r.created_at DESC")
            }
            ReadingSortKey::FinishedAt => {
                format!("r.finished_at {dir_sql}, r.created_at DESC")
            }
            ReadingSortKey::BookTitle => {
                format!("LOWER(bk.title) {dir_sql}, r.created_at DESC")
            }
        }
    }

    pub fn decode_quick_reviews(raw: Option<String>) -> Vec<QuickReview> {
        match raw {
            Some(s) if !s.is_empty() => serde_json::from_str::<Vec<String>>(&s)
                .unwrap_or_default()
                .iter()
                .filter_map(|v| QuickReview::from_str_value(v))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn encode_quick_reviews(reviews: &[QuickReview]) -> Option<String> {
        if reviews.is_empty() {
            None
        } else {
            let labels: Vec<&str> = reviews.iter().map(|r| r.label()).collect();
            serde_json::to_string(&labels).ok()
        }
    }

    fn to_domain(record: ReadingRecord) -> Result<Reading, RepositoryError> {
        let status = ReadingStatus::from_str(&record.status).map_err(|()| {
            RepositoryError::unexpected(format!("invalid reading status: {}", record.status))
        })?;
        let format = record
            .format
            .as_deref()
            .map(|s| {
                ReadingFormat::from_str(s).map_err(|()| {
                    RepositoryError::unexpected(format!("invalid reading format: {s}"))
                })
            })
            .transpose()?;

        Ok(Reading {
            id: ReadingId::new(record.id),
            user_id: UserId::new(record.user_id),
            book_id: BookId::new(record.book_id),
            status,
            format,
            started_at: record.started_at,
            finished_at: record.finished_at,
            rating: record.rating,
            quick_reviews: Self::decode_quick_reviews(record.review),
            created_at: record.created_at,
            updated_at: record.updated_at,
        })
    }

    fn to_domain_with_book(
        record: ReadingWithBookRecord,
    ) -> Result<ReadingWithBook, RepositoryError> {
        let status = ReadingStatus::from_str(&record.status).map_err(|()| {
            RepositoryError::unexpected(format!("invalid reading status: {}", record.status))
        })?;
        let format = record
            .format
            .as_deref()
            .map(|s| {
                ReadingFormat::from_str(s).map_err(|()| {
                    RepositoryError::unexpected(format!("invalid reading format: {s}"))
                })
            })
            .transpose()?;
        Ok(ReadingWithBook {
            reading: Reading {
                id: ReadingId::new(record.id),
                user_id: UserId::new(record.user_id),
                book_id: BookId::new(record.book_id),
                status,
                format,
                started_at: record.started_at,
                finished_at: record.finished_at,
                rating: record.rating,
                quick_reviews: Self::decode_quick_reviews(record.review),
                created_at: record.created_at,
                updated_at: record.updated_at,
            },
            book_title: record.book_title,
            author_names: record.author_names,
            page_count: record.page_count,
            year_published: record.year_published,
            primary_genre: record.primary_genre,
            secondary_genre: record.secondary_genre,
        })
    }

    fn push_filter(
        qb: &mut QueryBuilder<'_, crate::infrastructure::database::DatabaseDriver>,
        filter: &ReadingFilter,
    ) -> bool {
        let mut has_condition = false;

        if let Some(user_id) = filter.user_id {
            qb.push(" WHERE r.user_id = ");
            qb.push_bind(user_id.into_inner());
            has_condition = true;
        }

        if let Some(status) = &filter.status {
            qb.push(if has_condition {
                " AND r.status = "
            } else {
                " WHERE r.status = "
            });
            qb.push_bind(status.as_str().to_string());
            has_condition = true;
        }

        if let Some(book_id) = filter.book_id {
            qb.push(if has_condition {
                " AND r.book_id = "
            } else {
                " WHERE r.book_id = "
            });
            qb.push_bind(book_id.into_inner());
            has_condition = true;
        }

        has_condition
    }
}

#[async_trait]
impl ReadingRepository for SqlReadingRepository {
    async fn insert(&self, reading: NewReading) -> Result<Reading, RepositoryError> {
        let created_at = reading.created_at.unwrap_or_else(Utc::now);
        let query = r"
            INSERT INTO readings (user_id, book_id, status, format, started_at, finished_at, rating, review, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id, user_id, book_id, status, format, started_at, finished_at, rating, review, created_at, updated_at
        ";

        let record = query_as::<_, ReadingRecord>(query)
            .bind(reading.user_id.into_inner())
            .bind(reading.book_id.into_inner())
            .bind(reading.status.as_str())
            .bind(reading.format.map(|f| f.as_str().to_string()))
            .bind(reading.started_at)
            .bind(reading.finished_at)
            .bind(reading.rating)
            .bind(Self::encode_quick_reviews(&reading.quick_reviews))
            .bind(created_at)
            .bind(created_at)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        Self::to_domain(record)
    }

    async fn get(&self, id: ReadingId) -> Result<Reading, RepositoryError> {
        let query = r"
            SELECT id, user_id, book_id, status, format, started_at, finished_at, rating, review, created_at, updated_at
            FROM readings
            WHERE id = ?
        ";

        let record = query_as::<_, ReadingRecord>(query)
            .bind(id.into_inner())
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?
            .ok_or(RepositoryError::NotFound)?;

        Self::to_domain(record)
    }

    async fn get_with_book(&self, id: ReadingId) -> Result<ReadingWithBook, RepositoryError> {
        let query = format!("{BASE_SELECT} WHERE r.id = ? {BASE_GROUP_BY}");

        let record = query_as::<_, ReadingWithBookRecord>(&query)
            .bind(id.into_inner())
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?
            .ok_or(RepositoryError::NotFound)?;

        Self::to_domain_with_book(record)
    }

    async fn list(
        &self,
        filter: ReadingFilter,
        request: &ListRequest<ReadingSortKey>,
        search: Option<&str>,
    ) -> Result<Page<ReadingWithBook>, RepositoryError> {
        use crate::domain::listing::PageSize;
        use crate::infrastructure::repositories::pagination::{
            SearchFilter, push_search_condition,
        };

        let order_clause = Self::order_clause(request);
        let sf = search.and_then(|t| SearchFilter::new(t, vec!["bk.title", "a.name"]));

        match request.page_size() {
            PageSize::All => {
                let mut qb = QueryBuilder::new(BASE_SELECT);
                let has_where = Self::push_filter(&mut qb, &filter);
                if let Some(sf) = &sf {
                    push_search_condition(&mut qb, sf, has_where);
                }
                qb.push(BASE_GROUP_BY);
                qb.push(" ORDER BY ");
                qb.push(&order_clause);

                let records: Vec<ReadingWithBookRecord> = qb
                    .build_query_as()
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

                let mut items = Vec::with_capacity(records.len());
                for record in records {
                    items.push(Self::to_domain_with_book(record)?);
                }
                let total = items.len() as u64;
                let page_size = total.min(u64::from(u32::MAX)) as u32;
                Ok(Page::new(items, 1, page_size.max(1), total, true))
            }
            PageSize::Limited(page_size) => {
                let count_base = if sf.is_some() {
                    r"SELECT COUNT(DISTINCT r.id) FROM readings r
                        JOIN books bk ON r.book_id = bk.id
                        LEFT JOIN book_authors ba ON ba.book_id = bk.id
                        LEFT JOIN authors a ON a.id = ba.author_id"
                } else {
                    "SELECT COUNT(*) FROM readings r"
                };
                let mut count_qb = QueryBuilder::new(count_base);
                let has_where = Self::push_filter(&mut count_qb, &filter);
                if let Some(sf) = &sf {
                    push_search_condition(&mut count_qb, sf, has_where);
                }
                let (total,): (i64,) = count_qb
                    .build_query_as()
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

                let limit = i64::from(page_size);
                let adjusted = (*request).ensure_page_within(total as u64);
                let page = adjusted.page();
                let offset = i64::from(page - 1).saturating_mul(limit);

                let mut qb = QueryBuilder::new(BASE_SELECT);
                let has_where = Self::push_filter(&mut qb, &filter);
                if let Some(sf) = &sf {
                    push_search_condition(&mut qb, sf, has_where);
                }
                qb.push(BASE_GROUP_BY);
                qb.push(" ORDER BY ");
                qb.push(&order_clause);
                qb.push(" LIMIT ");
                qb.push_bind(limit);
                qb.push(" OFFSET ");
                qb.push_bind(offset);

                let records: Vec<ReadingWithBookRecord> = qb
                    .build_query_as()
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

                let mut items = Vec::with_capacity(records.len());
                for record in records {
                    items.push(Self::to_domain_with_book(record)?);
                }

                Ok(Page::new(items, page, page_size, total as u64, false))
            }
        }
    }

    async fn update(
        &self,
        id: ReadingId,
        changes: UpdateReading,
    ) -> Result<Reading, RepositoryError> {
        let mut builder = QueryBuilder::new("UPDATE readings SET updated_at = CURRENT_TIMESTAMP");
        let mut sep = true; // Already have updated_at

        if let Some(book_id) = changes.book_id {
            builder.push(", book_id = ");
            builder.push_bind(book_id.into_inner());
            sep = true;
        }

        if let Some(status) = &changes.status {
            builder.push(", status = ");
            builder.push_bind(status.as_str().to_string());
            sep = true;
        }

        if let Some(format) = &changes.format {
            builder.push(", format = ");
            builder.push_bind(format.as_str().to_string());
            sep = true;
        }

        push_update_field!(builder, sep, "started_at", changes.started_at);
        push_update_field!(builder, sep, "finished_at", changes.finished_at);
        push_update_field!(builder, sep, "rating", changes.rating);
        if let Some(ref reviews) = changes.quick_reviews {
            builder.push(", review = ");
            builder.push_bind(Self::encode_quick_reviews(reviews));
        }
        push_update_field!(builder, sep, "created_at", changes.created_at);
        let _ = sep; // Suppress unused_assignments warning from macro

        builder.push(" WHERE id = ");
        builder.push_bind(id.into_inner());
        builder.push(" RETURNING id, user_id, book_id, status, format, started_at, finished_at, rating, review, created_at, updated_at");

        let record = builder
            .build_query_as::<ReadingRecord>()
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?
            .ok_or(RepositoryError::NotFound)?;

        Self::to_domain(record)
    }

    async fn delete(&self, id: ReadingId) -> Result<(), RepositoryError> {
        let query = "DELETE FROM readings WHERE id = ?";

        let result = sqlx::query(query)
            .bind(id.into_inner())
            .execute(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct ReadingRecord {
    id: i64,
    user_id: i64,
    book_id: i64,
    status: String,
    format: Option<String>,
    started_at: Option<NaiveDate>,
    finished_at: Option<NaiveDate>,
    rating: Option<f64>,
    review: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct ReadingWithBookRecord {
    id: i64,
    user_id: i64,
    book_id: i64,
    status: String,
    format: Option<String>,
    started_at: Option<NaiveDate>,
    finished_at: Option<NaiveDate>,
    rating: Option<f64>,
    review: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    book_title: String,
    page_count: Option<i32>,
    year_published: Option<i32>,
    primary_genre: Option<String>,
    secondary_genre: Option<String>,
    author_names: String,
}
