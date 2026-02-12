use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, query, query_as};

use crate::domain::RepositoryError;
use crate::domain::authors::{Author, AuthorSortKey, NewAuthor, UpdateAuthor};
use crate::domain::ids::{AuthorId, UserId};
use crate::domain::listing::{ListRequest, Page};
use crate::domain::repositories::AuthorRepository;
use crate::infrastructure::database::DatabasePool;
use crate::infrastructure::repositories::macros::push_update_field;

#[derive(Clone)]
pub struct SqlAuthorRepository {
    pool: DatabasePool,
}

impl SqlAuthorRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    fn order_clause(request: &ListRequest<AuthorSortKey>) -> String {
        let dir_sql = request.sort_direction().as_sql();

        match request.sort_key() {
            AuthorSortKey::CreatedAt => format!("created_at {dir_sql}, name ASC"),
            AuthorSortKey::Name => format!("LOWER(name) {dir_sql}, created_at DESC"),
        }
    }

    fn into_domain(record: AuthorRecord) -> Author {
        Author {
            id: AuthorId::from(record.id),
            name: record.name,
            created_at: record.created_at,
        }
    }
}

#[async_trait]
impl AuthorRepository for SqlAuthorRepository {
    async fn insert(&self, new_author: NewAuthor) -> Result<Author, RepositoryError> {
        let new_author = new_author.normalize();
        let created_at = new_author.created_at.unwrap_or_else(Utc::now);

        let record = query_as::<_, AuthorRecord>(
            "INSERT INTO authors (name, created_at) VALUES (?, ?)\
             RETURNING id, name, created_at",
        )
        .bind(&new_author.name)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|err| {
            if let sqlx::Error::Database(db_err) = &err
                && db_err.is_unique_violation()
            {
                return RepositoryError::conflict("An author with this name already exists");
            }
            RepositoryError::unexpected(err.to_string())
        })?;

        Ok(Self::into_domain(record))
    }

    async fn get(&self, id: AuthorId) -> Result<Author, RepositoryError> {
        let record =
            query_as::<_, AuthorRecord>("SELECT id, name, created_at FROM authors WHERE id = ?")
                .bind(i64::from(id))
                .fetch_optional(&self.pool)
                .await
                .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        match record {
            Some(record) => Ok(Self::into_domain(record)),
            None => Err(RepositoryError::NotFound),
        }
    }

    async fn get_by_name(&self, name: &str) -> Result<Author, RepositoryError> {
        let record = query_as::<_, AuthorRecord>(
            "SELECT id, name, created_at FROM authors WHERE LOWER(TRIM(name)) = LOWER(TRIM(?))",
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
        request: &ListRequest<AuthorSortKey>,
        search: Option<&str>,
    ) -> Result<Page<Author>, RepositoryError> {
        use crate::infrastructure::repositories::pagination::SearchFilter;

        let order_clause = Self::order_clause(request);
        let base_query = "SELECT id, name, created_at FROM authors";
        let count_query = "SELECT COUNT(*) FROM authors";
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

    async fn list_for_user_library(
        &self,
        user_id: UserId,
        request: &ListRequest<AuthorSortKey>,
        search: Option<&str>,
    ) -> Result<Page<Author>, RepositoryError> {
        use crate::infrastructure::repositories::pagination::SearchFilter;

        let order_clause = Self::order_clause(request);

        let base_query = format!(
            r"SELECT DISTINCT a.id, a.name, a.created_at
               FROM authors a
               JOIN book_authors ba ON ba.author_id = a.id
               JOIN user_books ub ON ub.book_id = ba.book_id
               WHERE ub.user_id = {} AND ub.shelf = 'library'",
            i64::from(user_id)
        );

        let count_query = format!(
            r"SELECT COUNT(DISTINCT a.id)
               FROM authors a
               JOIN book_authors ba ON ba.author_id = a.id
               JOIN user_books ub ON ub.book_id = ba.book_id
               WHERE ub.user_id = {} AND ub.shelf = 'library'",
            i64::from(user_id)
        );

        let sf = search.and_then(|t| SearchFilter::new(t, vec!["a.name"]));

        crate::infrastructure::repositories::pagination::paginate(
            &self.pool,
            request,
            &base_query,
            &count_query,
            &order_clause,
            sf.as_ref(),
            |record| Ok(Self::into_domain(record)),
        )
        .await
    }

    async fn update(&self, id: AuthorId, changes: UpdateAuthor) -> Result<Author, RepositoryError> {
        let mut builder = QueryBuilder::new("UPDATE authors SET ");
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

    async fn delete(&self, id: AuthorId) -> Result<(), RepositoryError> {
        let result = query("DELETE FROM authors WHERE id = ?")
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
struct AuthorRecord {
    id: i64,
    name: String,
    created_at: DateTime<Utc>,
}
