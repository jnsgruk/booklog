use std::collections::HashSet;
use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, query_as};

use crate::domain::RepositoryError;
use crate::domain::book_items::{Book, BookWithAuthors};
use crate::domain::books::readings::ReadingStatus;
use crate::domain::ids::{BookId, GenreId, UserBookId, UserId};
use crate::domain::listing::{ListRequest, Page};
use crate::domain::repositories::UserBookRepository;
use crate::domain::user_books::{
    NewUserBook, ReadingSummary, Shelf, UserBook, UserBookSortKey, UserBookWithDetails,
};
use crate::infrastructure::database::DatabasePool;

#[derive(Clone)]
pub struct SqlUserBookRepository {
    pool: DatabasePool,
}

impl SqlUserBookRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    fn order_clause(request: &ListRequest<UserBookSortKey>) -> String {
        let dir_sql = request.sort_direction().as_sql();

        match request.sort_key() {
            UserBookSortKey::CreatedAt => format!("ub.created_at {dir_sql}, ub.id DESC"),
            UserBookSortKey::Title => format!("LOWER(b.title) {dir_sql}, ub.created_at DESC"),
            UserBookSortKey::Author => {
                format!("LOWER(COALESCE(ba_sort.author_name, '')) {dir_sql}, ub.created_at DESC")
            }
            UserBookSortKey::Genre => {
                format!("LOWER(COALESCE(pg.name, '')) {dir_sql}, ub.created_at DESC")
            }
            UserBookSortKey::Club => {
                format!("ub.book_club {dir_sql}, ub.created_at DESC")
            }
            UserBookSortKey::Status => {
                format!("COALESCE(lr.status, 'on_shelf') {dir_sql}, ub.created_at DESC")
            }
        }
    }

    fn to_domain(record: UserBookRecord) -> Result<UserBook, RepositoryError> {
        let shelf = Shelf::from_str(&record.shelf).map_err(|()| {
            RepositoryError::unexpected(format!("invalid shelf value: {}", record.shelf))
        })?;

        Ok(UserBook {
            id: UserBookId::new(record.id),
            user_id: UserId::new(record.user_id),
            book_id: BookId::new(record.book_id),
            shelf,
            book_club: record.book_club,
            created_at: record.created_at,
        })
    }

    fn to_domain_with_details(
        record: UserBookWithDetailsRecord,
        author_records: &[super::book_authors::BookAuthorRecord],
    ) -> Result<UserBookWithDetails, RepositoryError> {
        let shelf = Shelf::from_str(&record.shelf).map_err(|()| {
            RepositoryError::unexpected(format!("invalid shelf value: {}", record.shelf))
        })?;
        let authors = author_records
            .iter()
            .filter(|r| r.book_id == record.book_id)
            .map(super::book_authors::BookAuthorRecord::to_info)
            .collect();

        let reading_summary = match (record.reading_id, record.reading_status) {
            (Some(rid), Some(status_str)) => {
                let status = ReadingStatus::from_str(&status_str).map_err(|()| {
                    RepositoryError::unexpected(format!("invalid reading status: {status_str}"))
                })?;
                Some(ReadingSummary {
                    reading_id: crate::domain::ids::ReadingId::new(rid),
                    status,
                    started_at: record.reading_started_at,
                    finished_at: record.reading_finished_at,
                })
            }
            _ => None,
        };

        Ok(UserBookWithDetails {
            user_book: UserBook {
                id: UserBookId::new(record.id),
                user_id: UserId::new(record.user_id),
                book_id: BookId::new(record.book_id),
                shelf,
                book_club: record.book_club,
                created_at: record.ub_created_at,
            },
            book: BookWithAuthors {
                book: Book {
                    id: BookId::new(record.book_id),
                    title: record.title,
                    isbn: record.isbn,
                    description: record.description,
                    page_count: record.page_count,
                    year_published: record.year_published,
                    publisher: record.publisher,
                    language: record.language,
                    primary_genre_id: record.primary_genre_id.map(GenreId::new),
                    secondary_genre_id: record.secondary_genre_id.map(GenreId::new),
                    created_at: record.book_created_at,
                },
                authors,
                primary_genre: record.primary_genre,
                secondary_genre: record.secondary_genre,
            },
            reading_summary,
        })
    }

    async fn enrich_user_book_records(
        &self,
        records: Vec<UserBookWithDetailsRecord>,
    ) -> Result<Vec<UserBookWithDetails>, RepositoryError> {
        let book_ids: Vec<i64> = records.iter().map(|r| r.book_id).collect();
        let author_records =
            super::book_authors::fetch_authors_for_books(&self.pool, &book_ids).await?;
        let mut items = Vec::with_capacity(records.len());
        for record in records {
            items.push(Self::to_domain_with_details(record, &author_records)?);
        }
        Ok(items)
    }

    #[allow(clippy::similar_names)] // self vs shelf
    async fn count_user_books(
        &self,
        user_id: UserId,
        shelf: Option<&Shelf>,
        search_filter: Option<&crate::infrastructure::repositories::pagination::SearchFilter>,
    ) -> Result<i64, RepositoryError> {
        use crate::infrastructure::repositories::pagination::push_search_condition;

        let count_base = if search_filter.is_some() {
            r"SELECT COUNT(*) FROM user_books ub
               JOIN books b ON b.id = ub.book_id
               LEFT JOIN genres pg ON pg.id = b.primary_genre_id
               LEFT JOIN genres sg ON sg.id = b.secondary_genre_id
               WHERE ub.user_id = "
        } else {
            "SELECT COUNT(*) FROM user_books ub WHERE ub.user_id = "
        };
        let mut builder = QueryBuilder::new(count_base);
        builder.push_bind(user_id.into_inner());
        if let Some(s) = shelf {
            builder.push(" AND ub.shelf = ");
            builder.push_bind(s.as_str().to_string());
        }
        if let Some(sf) = search_filter {
            push_search_condition(&mut builder, sf, true);
        }
        let (total,): (i64,) = builder
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?;
        Ok(total)
    }
}

const USER_BOOKS_BASE_SELECT: &str = r"SELECT ub.id, ub.user_id, ub.book_id, ub.shelf, ub.book_club, ub.created_at AS ub_created_at,
                      b.title, b.isbn, b.description, b.page_count, b.year_published,
                      b.publisher, b.language, b.primary_genre_id, b.secondary_genre_id,
                      pg.name AS primary_genre, sg.name AS secondary_genre,
                      b.created_at AS book_created_at,
                      lr.id AS reading_id, lr.status AS reading_status,
                      lr.started_at AS reading_started_at, lr.finished_at AS reading_finished_at
               FROM user_books ub
               JOIN books b ON b.id = ub.book_id
               LEFT JOIN genres pg ON pg.id = b.primary_genre_id
               LEFT JOIN genres sg ON sg.id = b.secondary_genre_id
               LEFT JOIN (
                   SELECT book_id, MIN(rowid) AS min_rowid, author_id
                   FROM book_authors
                   GROUP BY book_id
               ) ba_first ON ba_first.book_id = b.id
               LEFT JOIN authors ba_sort ON ba_sort.id = ba_first.author_id
               LEFT JOIN (
                   SELECT r1.*
                   FROM readings r1
                   INNER JOIN (
                       SELECT book_id, user_id, MAX(created_at) AS max_created
                       FROM readings
                       GROUP BY book_id, user_id
                   ) r2 ON r1.book_id = r2.book_id AND r1.user_id = r2.user_id AND r1.created_at = r2.max_created
               ) lr ON lr.book_id = ub.book_id AND lr.user_id = ub.user_id
               WHERE ub.user_id = ";

#[async_trait]
impl UserBookRepository for SqlUserBookRepository {
    async fn insert(&self, user_book: NewUserBook) -> Result<UserBook, RepositoryError> {
        let query = r"
            INSERT INTO user_books (user_id, book_id, shelf, book_club)
            VALUES (?, ?, ?, ?)
            RETURNING id, user_id, book_id, shelf, book_club, created_at
        ";

        let record = query_as::<_, UserBookRecord>(query)
            .bind(user_book.user_id.into_inner())
            .bind(user_book.book_id.into_inner())
            .bind(user_book.shelf.as_str())
            .bind(user_book.book_club)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| {
                if let sqlx::Error::Database(db_err) = &err
                    && db_err.is_unique_violation()
                {
                    return RepositoryError::conflict("This book is already on the shelf");
                }
                RepositoryError::unexpected(err.to_string())
            })?;

        Self::to_domain(record)
    }

    async fn get(&self, id: UserBookId) -> Result<UserBook, RepositoryError> {
        let query = r"SELECT id, user_id, book_id, shelf, book_club, created_at FROM user_books WHERE id = ?";

        let record = query_as::<_, UserBookRecord>(query)
            .bind(id.into_inner())
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?
            .ok_or(RepositoryError::NotFound)?;

        Self::to_domain(record)
    }

    async fn get_by_user_and_book(
        &self,
        user_id: UserId,
        book_id: BookId,
    ) -> Result<UserBook, RepositoryError> {
        let query = r"SELECT id, user_id, book_id, shelf, book_club, created_at FROM user_books WHERE user_id = ? AND book_id = ?";

        let record = query_as::<_, UserBookRecord>(query)
            .bind(user_id.into_inner())
            .bind(book_id.into_inner())
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?
            .ok_or(RepositoryError::NotFound)?;

        Self::to_domain(record)
    }

    #[allow(clippy::similar_names)] // self vs shelf
    async fn list_by_user(
        &self,
        user_id: UserId,
        shelf: Option<Shelf>,
        request: &ListRequest<UserBookSortKey>,
        search: Option<&str>,
    ) -> Result<Page<UserBookWithDetails>, RepositoryError> {
        use crate::domain::listing::PageSize;
        use crate::infrastructure::repositories::pagination::{
            SearchFilter, push_search_condition,
        };

        let order_clause = Self::order_clause(request);
        let search_filter = search.and_then(|t| {
            SearchFilter::new(
                t,
                vec![
                    "b.title",
                    "COALESCE(b.publisher,'')",
                    "COALESCE(pg.name,'')",
                    "COALESCE(sg.name,'')",
                    "COALESCE(b.isbn,'')",
                ],
            )
        });

        let push_filters =
            |builder: &mut QueryBuilder<'_, crate::infrastructure::database::DatabaseDriver>| {
                builder.push_bind(user_id.into_inner());
                if let Some(s) = &shelf {
                    builder.push(" AND ub.shelf = ");
                    builder.push_bind(s.as_str().to_string());
                }
            };

        let build_data_query = |limit_offset: Option<(i64, i64)>| {
            let mut builder = QueryBuilder::new(USER_BOOKS_BASE_SELECT);
            push_filters(&mut builder);
            if let Some(sf) = &search_filter {
                push_search_condition(&mut builder, sf, true);
            }
            builder.push(" ORDER BY ");
            builder.push(&order_clause);
            if let Some((limit, offset)) = limit_offset {
                builder.push(" LIMIT ");
                builder.push_bind(limit);
                builder.push(" OFFSET ");
                builder.push_bind(offset);
            }
            builder
        };

        match request.page_size() {
            PageSize::All => {
                let records: Vec<UserBookWithDetailsRecord> = build_data_query(None)
                    .build_query_as()
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

                let items = self.enrich_user_book_records(records).await?;
                let total = items.len() as u64;
                let size = total.min(u64::from(u32::MAX)) as u32;
                Ok(Page::new(items, 1, size.max(1), total, true))
            }
            PageSize::Limited(page_size) => {
                let total = self
                    .count_user_books(user_id, shelf.as_ref(), search_filter.as_ref())
                    .await?;

                let limit = i64::from(page_size);
                let adjusted = (*request).ensure_page_within(total as u64);
                let current_page = adjusted.page();
                let offset = i64::from(current_page - 1).saturating_mul(limit);

                let records: Vec<UserBookWithDetailsRecord> =
                    build_data_query(Some((limit, offset)))
                        .build_query_as()
                        .fetch_all(&self.pool)
                        .await
                        .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

                let items = self.enrich_user_book_records(records).await?;
                Ok(Page::new(
                    items,
                    current_page,
                    page_size,
                    total as u64,
                    false,
                ))
            }
        }
    }

    #[allow(clippy::similar_names)] // self vs shelf
    async fn move_shelf(&self, id: UserBookId, shelf: Shelf) -> Result<UserBook, RepositoryError> {
        let query = r"UPDATE user_books SET shelf = ? WHERE id = ? RETURNING id, user_id, book_id, shelf, book_club, created_at";

        let record = query_as::<_, UserBookRecord>(query)
            .bind(shelf.as_str())
            .bind(id.into_inner())
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?
            .ok_or(RepositoryError::NotFound)?;

        Self::to_domain(record)
    }

    async fn set_book_club(
        &self,
        id: UserBookId,
        book_club: bool,
    ) -> Result<UserBook, RepositoryError> {
        let query = r"UPDATE user_books SET book_club = ? WHERE id = ? RETURNING id, user_id, book_id, shelf, book_club, created_at";

        let record = query_as::<_, UserBookRecord>(query)
            .bind(book_club)
            .bind(id.into_inner())
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?
            .ok_or(RepositoryError::NotFound)?;

        Self::to_domain(record)
    }

    async fn delete(&self, id: UserBookId) -> Result<(), RepositoryError> {
        let result = sqlx::query("DELETE FROM user_books WHERE id = ?")
            .bind(id.into_inner())
            .execute(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }

    #[allow(clippy::similar_names)] // self vs shelf
    async fn book_ids_for_user(
        &self,
        user_id: UserId,
        shelf: Option<Shelf>,
    ) -> Result<HashSet<BookId>, RepositoryError> {
        let mut qb =
            sqlx::query_as::<_, (i64,)>("SELECT book_id FROM user_books WHERE user_id = ?")
                .bind(user_id.into_inner());

        if let Some(s) = shelf {
            qb = sqlx::query_as::<_, (i64,)>(
                "SELECT book_id FROM user_books WHERE user_id = ? AND shelf = ?",
            )
            .bind(user_id.into_inner())
            .bind(s.as_str());
        }

        let rows = qb
            .fetch_all(&self.pool)
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        Ok(rows.into_iter().map(|(id,)| BookId::from(id)).collect())
    }
}

#[derive(sqlx::FromRow)]
struct UserBookRecord {
    id: i64,
    user_id: i64,
    book_id: i64,
    shelf: String,
    book_club: bool,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct UserBookWithDetailsRecord {
    id: i64,
    user_id: i64,
    book_id: i64,
    shelf: String,
    book_club: bool,
    ub_created_at: DateTime<Utc>,
    title: String,
    isbn: Option<String>,
    description: Option<String>,
    page_count: Option<i32>,
    year_published: Option<i32>,
    publisher: Option<String>,
    language: Option<String>,
    primary_genre_id: Option<i64>,
    secondary_genre_id: Option<i64>,
    primary_genre: Option<String>,
    secondary_genre: Option<String>,
    book_created_at: DateTime<Utc>,
    reading_id: Option<i64>,
    reading_status: Option<String>,
    reading_started_at: Option<chrono::NaiveDate>,
    reading_finished_at: Option<chrono::NaiveDate>,
}
