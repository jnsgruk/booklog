use crate::domain::RepositoryError;
use crate::domain::ids::{TimelineEventId, UserId};
use crate::domain::listing::{ListRequest, Page};
use crate::domain::repositories::TimelineEventRepository;
use crate::domain::timeline::{
    NewTimelineEvent, TimelineEvent, TimelineEventDetail, TimelineReadingData, TimelineSortKey,
};
use crate::infrastructure::database::DatabasePool;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::from_str;

#[derive(Clone)]
pub struct SqlTimelineEventRepository {
    pool: DatabasePool,
}

impl SqlTimelineEventRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TimelineEventRepository for SqlTimelineEventRepository {
    async fn insert(&self, event: NewTimelineEvent) -> Result<TimelineEvent, RepositoryError> {
        let query = r"
            INSERT INTO timeline_events (user_id, entity_type, entity_id, action, occurred_at, title, details_json, genres_json, reading_data_json)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id, entity_type, entity_id, action, occurred_at, title, details_json, genres_json, reading_data_json
        ";

        let details_json = serde_json::to_string(&event.details).map_err(|err| {
            RepositoryError::unexpected(format!("failed to encode timeline event details: {err}"))
        })?;

        let genres_json = serde_json::to_string(&event.genres).map_err(|err| {
            RepositoryError::unexpected(format!("failed to encode timeline event genres: {err}"))
        })?;

        let reading_data_json = event
            .reading_data
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|err| {
                RepositoryError::unexpected(format!("failed to encode reading data: {err}"))
            })?;

        let record = sqlx::query_as::<_, TimelineEventRecord>(query)
            .bind(event.user_id.map(crate::domain::ids::UserId::into_inner))
            .bind(event.entity_type)
            .bind(event.entity_id)
            .bind(event.action)
            .bind(event.occurred_at)
            .bind(event.title)
            .bind(details_json)
            .bind(genres_json)
            .bind(reading_data_json)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        record.into_domain()
    }

    async fn list(
        &self,
        user_id: Option<UserId>,
        request: &ListRequest<TimelineSortKey>,
    ) -> Result<Page<TimelineEvent>, RepositoryError> {
        use crate::domain::listing::PageSize;
        use sqlx::QueryBuilder;

        let direction_sql = request.sort_direction().as_sql();
        let order_clause = format!("occurred_at {direction_sql}, id DESC");
        let uid_val = user_id.map(UserId::into_inner);

        let base_select = r"SELECT
            id, entity_type, entity_id, action, occurred_at, title,
            details_json, genres_json, reading_data_json
        FROM timeline_events";

        match request.page_size() {
            PageSize::All => {
                let mut qb = QueryBuilder::new(base_select);
                if let Some(uid) = uid_val {
                    qb.push(" WHERE user_id = ");
                    qb.push_bind(uid);
                }
                qb.push(" ORDER BY ");
                qb.push(&order_clause);

                let records: Vec<TimelineEventRecord> = qb
                    .build_query_as()
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

                let mut items = Vec::with_capacity(records.len());
                for record in records {
                    items.push(record.into_domain()?);
                }
                let total = items.len() as u64;
                let page_size = total.min(u64::from(u32::MAX)) as u32;
                Ok(Page::new(items, 1, page_size.max(1), total, true))
            }
            PageSize::Limited(page_size) => {
                let mut count_qb = QueryBuilder::new("SELECT COUNT(*) FROM timeline_events");
                if let Some(uid) = uid_val {
                    count_qb.push(" WHERE user_id = ");
                    count_qb.push_bind(uid);
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

                let mut qb = QueryBuilder::new(base_select);
                if let Some(uid) = uid_val {
                    qb.push(" WHERE user_id = ");
                    qb.push_bind(uid);
                }
                qb.push(" ORDER BY ");
                qb.push(&order_clause);
                qb.push(" LIMIT ");
                qb.push_bind(limit);
                qb.push(" OFFSET ");
                qb.push_bind(offset);

                let records: Vec<TimelineEventRecord> = qb
                    .build_query_as()
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

                let mut items = Vec::with_capacity(records.len());
                for record in records {
                    items.push(record.into_domain()?);
                }

                Ok(Page::new(items, page, page_size, total as u64, false))
            }
        }
    }
}

#[derive(sqlx::FromRow)]
struct TimelineEventRecord {
    id: i64,
    entity_type: String,
    entity_id: i64,
    action: String,
    occurred_at: DateTime<Utc>,
    title: String,
    details_json: Option<String>,
    genres_json: Option<String>,
    reading_data_json: Option<String>,
}

impl TimelineEventRecord {
    fn into_domain(self) -> Result<TimelineEvent, RepositoryError> {
        let details = match self.details_json {
            Some(raw) if !raw.is_empty() => {
                from_str::<Vec<TimelineEventDetail>>(&raw).map_err(|err| {
                    RepositoryError::unexpected(format!(
                        "failed to decode timeline event details: {err}"
                    ))
                })?
            }
            _ => Vec::new(),
        };

        let genres = match self.genres_json {
            Some(raw) if !raw.is_empty() => from_str::<Vec<String>>(&raw).map_err(|err| {
                RepositoryError::unexpected(format!(
                    "failed to decode timeline event genres: {err}"
                ))
            })?,
            _ => Vec::new(),
        };

        let reading_data = match self.reading_data_json {
            Some(raw) if !raw.is_empty() => {
                Some(from_str::<TimelineReadingData>(&raw).map_err(|err| {
                    RepositoryError::unexpected(format!("failed to decode reading data: {err}"))
                })?)
            }
            _ => None,
        };

        Ok(TimelineEvent {
            id: TimelineEventId::from(self.id),
            entity_type: self.entity_type,
            entity_id: self.entity_id,
            action: self.action,
            occurred_at: self.occurred_at,
            title: self.title,
            details,
            genres,
            reading_data,
        })
    }
}
