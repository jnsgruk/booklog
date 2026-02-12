use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, query, query_as};

use crate::domain::RepositoryError;
use crate::domain::genres::{Genre, GenreSortKey, NewGenre, UpdateGenre};
use crate::domain::ids::GenreId;
use crate::domain::listing::{ListRequest, Page};
use crate::domain::repositories::GenreRepository;
use crate::infrastructure::database::DatabasePool;
use crate::infrastructure::repositories::macros::push_update_field;

#[derive(Clone)]
pub struct SqlGenreRepository {
    pool: DatabasePool,
}

impl SqlGenreRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    fn order_clause(request: &ListRequest<GenreSortKey>) -> String {
        let dir_sql = request.sort_direction().as_sql();

        match request.sort_key() {
            GenreSortKey::CreatedAt => format!("created_at {dir_sql}, name ASC"),
            GenreSortKey::Name => format!("LOWER(name) {dir_sql}, created_at DESC"),
        }
    }

    fn into_domain(record: GenreRecord) -> Genre {
        Genre {
            id: GenreId::from(record.id),
            name: record.name,
            created_at: record.created_at,
        }
    }
}

#[async_trait]
impl GenreRepository for SqlGenreRepository {
    async fn insert(&self, new_genre: NewGenre) -> Result<Genre, RepositoryError> {
        let new_genre = new_genre.normalize();
        let created_at = new_genre.created_at.unwrap_or_else(Utc::now);

        let record = query_as::<_, GenreRecord>(
            "INSERT INTO genres (name, created_at) VALUES (?, ?)\
             RETURNING id, name, created_at",
        )
        .bind(&new_genre.name)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|err| {
            if let sqlx::Error::Database(db_err) = &err
                && db_err.is_unique_violation()
            {
                return RepositoryError::conflict("A genre with this name already exists");
            }
            RepositoryError::unexpected(err.to_string())
        })?;

        Ok(Self::into_domain(record))
    }

    async fn get(&self, id: GenreId) -> Result<Genre, RepositoryError> {
        let record =
            query_as::<_, GenreRecord>("SELECT id, name, created_at FROM genres WHERE id = ?")
                .bind(i64::from(id))
                .fetch_optional(&self.pool)
                .await
                .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        match record {
            Some(record) => Ok(Self::into_domain(record)),
            None => Err(RepositoryError::NotFound),
        }
    }

    async fn get_by_name(&self, name: &str) -> Result<Genre, RepositoryError> {
        let record = query_as::<_, GenreRecord>(
            "SELECT id, name, created_at FROM genres WHERE LOWER(TRIM(name)) = LOWER(TRIM(?))",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        match record {
            Some(record) => Ok(Self::into_domain(record)),
            None => Err(RepositoryError::NotFound),
        }
    }

    async fn list(
        &self,
        request: &ListRequest<GenreSortKey>,
        search: Option<&str>,
    ) -> Result<Page<Genre>, RepositoryError> {
        use crate::infrastructure::repositories::pagination::SearchFilter;

        let order_clause = Self::order_clause(request);
        let base_query = "SELECT id, name, created_at FROM genres";
        let count_query = "SELECT COUNT(*) FROM genres";
        let sf = search.and_then(|t| SearchFilter::new(t, vec!["name"]));

        crate::infrastructure::repositories::pagination::paginate(
            &self.pool,
            request,
            base_query,
            count_query,
            &order_clause,
            sf.as_ref(),
            |record| Ok(Self::into_domain(record)),
        )
        .await
    }

    async fn update(&self, id: GenreId, changes: UpdateGenre) -> Result<Genre, RepositoryError> {
        let mut builder = QueryBuilder::new("UPDATE genres SET ");
        let mut sep = false;

        push_update_field!(builder, sep, "name", changes.name);
        push_update_field!(builder, sep, "created_at", changes.created_at);

        if !sep {
            return Err(RepositoryError::unexpected(
                "No fields provided for update".to_string(),
            ));
        }

        builder.push(" WHERE id = ");
        builder.push_bind(i64::from(id));

        let result = builder
            .build()
            .execute(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        self.get(id).await
    }

    async fn delete(&self, id: GenreId) -> Result<(), RepositoryError> {
        let result = query("DELETE FROM genres WHERE id = ?")
            .bind(i64::from(id))
            .execute(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct GenreRecord {
    id: i64,
    name: String,
    created_at: DateTime<Utc>,
}
