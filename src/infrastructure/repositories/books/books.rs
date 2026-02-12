use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, query, query_as};

use crate::domain::RepositoryError;
use crate::domain::book_items::{Book, BookSortKey, BookWithAuthors, NewBook, UpdateBook};
use crate::domain::ids::{AuthorId, BookId, GenreId};
use crate::domain::listing::{ListRequest, Page};
use crate::domain::repositories::BookRepository;
use crate::infrastructure::database::DatabasePool;
use crate::infrastructure::repositories::macros::push_update_field;

#[derive(Clone)]
pub struct SqlBookRepository {
    pool: DatabasePool,
}

impl SqlBookRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    fn order_clause(request: &ListRequest<BookSortKey>) -> String {
        let dir_sql = request.sort_direction().as_sql();

        match request.sort_key() {
            BookSortKey::CreatedAt => format!("b.created_at {dir_sql}, LOWER(b.title) ASC"),
            BookSortKey::Title => format!("LOWER(b.title) {dir_sql}, b.created_at DESC"),
            BookSortKey::Author => {
                format!("LOWER(COALESCE(ba_sort.author_name, '')) {dir_sql}, b.created_at DESC")
            }
            BookSortKey::YearPublished => {
                format!("COALESCE(b.year_published, 0) {dir_sql}, b.created_at DESC")
            }
            BookSortKey::Publisher => {
                format!("LOWER(COALESCE(b.publisher, '')) {dir_sql}, b.created_at DESC")
            }
        }
    }

    fn into_book(record: BookRecord) -> Book {
        Book {
            id: BookId::from(record.id),
            title: record.title,
            isbn: record.isbn,
            description: record.description,
            page_count: record.page_count,
            year_published: record.year_published,
            publisher: record.publisher,
            language: record.language,
            primary_genre_id: record.primary_genre_id.map(GenreId::from),
            secondary_genre_id: record.secondary_genre_id.map(GenreId::from),
            created_at: record.created_at,
        }
    }

    fn build_with_authors(
        book: Book,
        author_records: &[super::book_authors::BookAuthorRecord],
    ) -> BookWithAuthors {
        let authors = author_records
            .iter()
            .filter(|r| r.book_id == book.id.into_inner())
            .map(super::book_authors::BookAuthorRecord::to_info)
            .collect();

        BookWithAuthors {
            book,
            authors,
            primary_genre: None,
            secondary_genre: None,
        }
    }

    async fn fetch_genre_names(
        &self,
        book: &Book,
    ) -> Result<(Option<String>, Option<String>), RepositoryError> {
        let primary = if let Some(id) = book.primary_genre_id {
            sqlx::query_scalar::<_, String>("SELECT name FROM genres WHERE id = ?")
                .bind(i64::from(id))
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| RepositoryError::unexpected(e.to_string()))?
        } else {
            None
        };
        let secondary = if let Some(id) = book.secondary_genre_id {
            sqlx::query_scalar::<_, String>("SELECT name FROM genres WHERE id = ?")
                .bind(i64::from(id))
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| RepositoryError::unexpected(e.to_string()))?
        } else {
            None
        };
        Ok((primary, secondary))
    }

    async fn fetch_genre_names_for_books(
        &self,
        books: &[Book],
    ) -> Result<std::collections::HashMap<i64, String>, RepositoryError> {
        let genre_ids: Vec<i64> = books
            .iter()
            .flat_map(|b| [b.primary_genre_id, b.secondary_genre_id])
            .flatten()
            .map(GenreId::into_inner)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if genre_ids.is_empty() {
            return Ok(HashMap::default());
        }

        let mut qb = QueryBuilder::new("SELECT id, name FROM genres WHERE id IN (");
        let mut sep = qb.separated(", ");
        for id in &genre_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");

        let records = qb
            .build_query_as::<GenreNameRecord>()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| RepositoryError::unexpected(e.to_string()))?;

        Ok(records.into_iter().map(|r| (r.id, r.name)).collect())
    }

    fn enrich_genre_names(
        bwa: &mut BookWithAuthors,
        genre_map: &std::collections::HashMap<i64, String>,
    ) {
        bwa.primary_genre = bwa
            .book
            .primary_genre_id
            .and_then(|id| genre_map.get(&id.into_inner()).cloned());
        bwa.secondary_genre = bwa
            .book
            .secondary_genre_id
            .and_then(|id| genre_map.get(&id.into_inner()).cloned());
    }

    async fn enrich_books(
        &self,
        books: Vec<Book>,
    ) -> Result<Vec<BookWithAuthors>, RepositoryError> {
        let book_ids: Vec<i64> = books.iter().map(|b| b.id.into_inner()).collect();
        let author_records =
            super::book_authors::fetch_authors_for_books(&self.pool, &book_ids).await?;
        let genre_map = self.fetch_genre_names_for_books(&books).await?;

        Ok(books
            .into_iter()
            .map(|book| {
                let mut bwa = Self::build_with_authors(book, &author_records);
                Self::enrich_genre_names(&mut bwa, &genre_map);
                bwa
            })
            .collect())
    }
}

#[async_trait]
impl BookRepository for SqlBookRepository {
    async fn insert(&self, new_book: NewBook) -> Result<Book, RepositoryError> {
        let created_at = new_book.created_at.unwrap_or_else(Utc::now);

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        let record = query_as::<_, BookRecord>(
            r"INSERT INTO books (title, isbn, description, page_count, year_published, publisher, language, primary_genre_id, secondary_genre_id, created_at)
              VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
              RETURNING id, title, isbn, description, page_count, year_published, publisher, language, primary_genre_id, secondary_genre_id, created_at",
        )
        .bind(&new_book.title)
        .bind(new_book.isbn.as_deref())
        .bind(new_book.description.as_deref())
        .bind(new_book.page_count)
        .bind(new_book.year_published)
        .bind(new_book.publisher.as_deref())
        .bind(new_book.language.as_deref())
        .bind(new_book.primary_genre_id.map(GenreId::into_inner))
        .bind(new_book.secondary_genre_id.map(GenreId::into_inner))
        .bind(created_at)
        .fetch_one(&mut *tx)
        .await
        .map_err(|err| {
            if let sqlx::Error::Database(db_err) = &err
                && db_err.is_unique_violation()
            {
                return RepositoryError::conflict("A book with this title already exists");
            }
            RepositoryError::unexpected(err.to_string())
        })?;

        let book_id = record.id;

        for ba in &new_book.authors {
            query("INSERT INTO book_authors (book_id, author_id, role) VALUES (?, ?, ?)")
                .bind(book_id)
                .bind(i64::from(ba.author_id))
                .bind(ba.role.as_str())
                .execute(&mut *tx)
                .await
                .map_err(|err| {
                    if err.to_string().contains("FOREIGN KEY constraint failed") {
                        return RepositoryError::NotFound;
                    }
                    RepositoryError::unexpected(err.to_string())
                })?;
        }

        tx.commit()
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        Ok(Self::into_book(record))
    }

    async fn get(&self, id: BookId) -> Result<Book, RepositoryError> {
        let record = query_as::<_, BookRecord>(
            r"SELECT id, title, isbn, description, page_count, year_published, publisher, language, primary_genre_id, secondary_genre_id, created_at
              FROM books WHERE id = ?",
        )
        .bind(i64::from(id))
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| RepositoryError::unexpected(err.to_string()))?
        .ok_or(RepositoryError::NotFound)?;

        Ok(Self::into_book(record))
    }

    async fn get_with_authors(&self, id: BookId) -> Result<BookWithAuthors, RepositoryError> {
        let book = self.get(id).await?;
        let author_records =
            super::book_authors::fetch_authors_for_books(&self.pool, &[book.id.into_inner()])
                .await?;
        let (primary_genre, secondary_genre) = self.fetch_genre_names(&book).await?;
        let mut bwa = Self::build_with_authors(book, &author_records);
        bwa.primary_genre = primary_genre;
        bwa.secondary_genre = secondary_genre;
        Ok(bwa)
    }

    async fn get_by_title(&self, title: &str) -> Result<Book, RepositoryError> {
        let record = query_as::<_, BookRecord>(
            r"SELECT id, title, isbn, description, page_count, year_published, publisher, language, primary_genre_id, secondary_genre_id, created_at
              FROM books WHERE LOWER(TRIM(title)) = LOWER(TRIM(?))",
        )
        .bind(title)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| RepositoryError::unexpected(err.to_string()))?
        .ok_or(RepositoryError::NotFound)?;

        Ok(Self::into_book(record))
    }

    async fn get_by_isbn(&self, isbn: &str) -> Result<Book, RepositoryError> {
        let record = query_as::<_, BookRecord>(
            r"SELECT id, title, isbn, description, page_count, year_published, publisher, language, primary_genre_id, secondary_genre_id, created_at
              FROM books WHERE isbn = ?",
        )
        .bind(isbn)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| RepositoryError::unexpected(err.to_string()))?
        .ok_or(RepositoryError::NotFound)?;

        Ok(Self::into_book(record))
    }

    async fn list(
        &self,
        request: &ListRequest<BookSortKey>,
        search: Option<&str>,
    ) -> Result<Page<BookWithAuthors>, RepositoryError> {
        use crate::infrastructure::repositories::pagination::SearchFilter;

        let order_clause = Self::order_clause(request);

        // Use LEFT JOINs to get the first author name for sorting and genre names for search
        let base_query = r"SELECT b.id, b.title, b.isbn, b.description, b.page_count, b.year_published, b.publisher, b.language, b.primary_genre_id, b.secondary_genre_id, b.created_at
              FROM books b
              LEFT JOIN (
                  SELECT book_id, MIN(rowid) AS min_rowid, author_id
                  FROM book_authors
                  GROUP BY book_id
              ) ba_first ON ba_first.book_id = b.id
              LEFT JOIN authors ba_sort ON ba_sort.id = ba_first.author_id
              LEFT JOIN genres pg ON pg.id = b.primary_genre_id
              LEFT JOIN genres sg ON sg.id = b.secondary_genre_id";
        let count_query = "SELECT COUNT(*) FROM books b";

        let sf = search.and_then(|t| {
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

        let page: Page<Book> = crate::infrastructure::repositories::pagination::paginate(
            &self.pool,
            request,
            base_query,
            count_query,
            &order_clause,
            sf.as_ref(),
            |r| Ok(Self::into_book(r)),
        )
        .await?;

        let items = self.enrich_books(page.items).await?;

        Ok(Page::new(
            items,
            page.page,
            page.page_size,
            page.total,
            page.showing_all,
        ))
    }

    async fn list_by_author(
        &self,
        author_id: AuthorId,
    ) -> Result<Vec<BookWithAuthors>, RepositoryError> {
        let records = query_as::<_, BookRecord>(
            r"SELECT b.id, b.title, b.isbn, b.description, b.page_count, b.year_published, b.publisher, b.language, b.primary_genre_id, b.secondary_genre_id, b.created_at
              FROM books b
              JOIN book_authors ba ON ba.book_id = b.id
              WHERE ba.author_id = ?
              ORDER BY b.created_at DESC",
        )
        .bind(i64::from(author_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        let books: Vec<Book> = records.into_iter().map(Self::into_book).collect();

        self.enrich_books(books).await
    }

    async fn list_by_genre(
        &self,
        genre_id: GenreId,
    ) -> Result<Vec<BookWithAuthors>, RepositoryError> {
        let records = query_as::<_, BookRecord>(
            r"SELECT b.id, b.title, b.isbn, b.description, b.page_count, b.year_published, b.publisher, b.language, b.primary_genre_id, b.secondary_genre_id, b.created_at
              FROM books b
              WHERE b.primary_genre_id = ? OR b.secondary_genre_id = ?
              ORDER BY b.created_at DESC",
        )
        .bind(i64::from(genre_id))
        .bind(i64::from(genre_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        let books: Vec<Book> = records.into_iter().map(Self::into_book).collect();

        self.enrich_books(books).await
    }

    async fn update(&self, id: BookId, changes: UpdateBook) -> Result<Book, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        let mut builder = QueryBuilder::new("UPDATE books SET ");
        let mut sep = false;

        push_update_field!(builder, sep, "title", changes.title);
        push_update_field!(builder, sep, "isbn", changes.isbn);
        push_update_field!(builder, sep, "description", changes.description);
        push_update_field!(builder, sep, "page_count", changes.page_count);
        push_update_field!(builder, sep, "year_published", changes.year_published);
        push_update_field!(builder, sep, "publisher", changes.publisher);
        push_update_field!(builder, sep, "language", changes.language);
        push_update_field!(builder, sep, "created_at", changes.created_at);

        if let Some(genre_id) = &changes.primary_genre_id {
            if sep {
                builder.push(", ");
            }
            sep = true;
            builder.push("primary_genre_id = ");
            builder.push_bind(genre_id.map(i64::from));
        }
        if let Some(genre_id) = &changes.secondary_genre_id {
            if sep {
                builder.push(", ");
            }
            sep = true;
            builder.push("secondary_genre_id = ");
            builder.push_bind(genre_id.map(i64::from));
        }

        let has_author_changes = changes.authors.is_some();

        if !sep && !has_author_changes {
            return Err(RepositoryError::unexpected(
                "No fields provided for update".to_string(),
            ));
        }

        if sep {
            builder.push(" WHERE id = ");
            builder.push_bind(i64::from(id));

            let result = builder
                .build()
                .execute(&mut *tx)
                .await
                .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

            if result.rows_affected() == 0 {
                return Err(RepositoryError::NotFound);
            }
        }

        if let Some(authors) = changes.authors {
            query("DELETE FROM book_authors WHERE book_id = ?")
                .bind(i64::from(id))
                .execute(&mut *tx)
                .await
                .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

            for ba in &authors {
                query("INSERT INTO book_authors (book_id, author_id, role) VALUES (?, ?, ?)")
                    .bind(i64::from(id))
                    .bind(i64::from(ba.author_id))
                    .bind(ba.role.as_str())
                    .execute(&mut *tx)
                    .await
                    .map_err(|err| RepositoryError::unexpected(err.to_string()))?;
            }
        }

        tx.commit()
            .await
            .map_err(|err| RepositoryError::unexpected(err.to_string()))?;

        self.get(id).await
    }

    async fn delete(&self, id: BookId) -> Result<(), RepositoryError> {
        let result = query("DELETE FROM books WHERE id = ?")
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

#[derive(sqlx::FromRow)]
struct BookRecord {
    id: i64,
    title: String,
    isbn: Option<String>,
    description: Option<String>,
    page_count: Option<i32>,
    year_published: Option<i32>,
    publisher: Option<String>,
    language: Option<String>,
    primary_genre_id: Option<i64>,
    secondary_genre_id: Option<i64>,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct GenreNameRecord {
    id: i64,
    name: String,
}
