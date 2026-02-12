use std::str::FromStr;

use anyhow::{Context, bail};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};

use crate::domain::authors::Author;
use crate::domain::book_items::Book;
use crate::domain::genres::Genre;
use crate::domain::ids::{AuthorId, BookId, GenreId, ReadingId, TimelineEventId, UserId};
use crate::domain::readings::{QuickReview, Reading, ReadingFormat, ReadingStatus};

fn encode_quick_reviews(reviews: &[QuickReview]) -> Option<String> {
    if reviews.is_empty() {
        None
    } else {
        let labels: Vec<&str> = reviews.iter().map(|r| r.label()).collect();
        serde_json::to_string(&labels).ok()
    }
}
use crate::domain::timeline::TimelineEvent;
use crate::infrastructure::database::{DatabasePool, DatabaseTransaction};

fn decode_json_vec<T: serde::de::DeserializeOwned>(
    raw: Option<String>,
    label: &str,
) -> anyhow::Result<Vec<T>> {
    match raw {
        Some(s) if !s.is_empty() => {
            from_str(&s).with_context(|| format!("failed to decode {label}: {s}"))
        }
        _ => Ok(Vec::new()),
    }
}

fn decode_json_opt<T: serde::de::DeserializeOwned>(
    raw: Option<String>,
    label: &str,
) -> anyhow::Result<Option<T>> {
    match raw {
        Some(s) if !s.is_empty() => from_str(&s)
            .map(Some)
            .with_context(|| format!("failed to decode {label}: {s}")),
        _ => Ok(None),
    }
}

mod base64_serde {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(data: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&STANDARD.encode(data))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupImage {
    pub entity_type: String,
    pub entity_id: i64,
    pub content_type: String,
    #[serde(with = "base64_serde")]
    pub image_data: Vec<u8>,
    #[serde(with = "base64_serde")]
    pub thumbnail_data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupBookAuthor {
    pub book_id: i64,
    pub author_id: i64,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupData {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub authors: Vec<Author>,
    #[serde(default)]
    pub genres: Vec<Genre>,
    pub books: Vec<Book>,
    #[serde(default)]
    pub book_authors: Vec<BackupBookAuthor>,
    pub readings: Vec<Reading>,
    pub timeline_events: Vec<TimelineEvent>,
    #[serde(default)]
    pub images: Vec<BackupImage>,
}

pub struct BackupService {
    pool: DatabasePool,
}

impl BackupService {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    pub async fn export(&self) -> anyhow::Result<BackupData> {
        // Use a transaction for snapshot isolation so all tables are read consistently
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin export transaction")?;

        let authors = self.export_authors(&mut tx).await?;
        let genres = self.export_genres(&mut tx).await?;
        let books = self.export_books(&mut tx).await?;
        let book_authors = self.export_book_authors(&mut tx).await?;
        let readings = self.export_readings(&mut tx).await?;
        let timeline_events = self.export_timeline_events(&mut tx).await?;
        let images = self.export_images(&mut tx).await?;

        tx.commit()
            .await
            .context("failed to commit export transaction")?;

        Ok(BackupData {
            version: 3,
            created_at: Utc::now(),
            authors,
            genres,
            books,
            book_authors,
            readings,
            timeline_events,
            images,
        })
    }

    pub async fn restore(&self, data: BackupData) -> anyhow::Result<()> {
        self.verify_empty_database().await?;

        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin transaction")?;

        self.restore_authors(&mut tx, &data.authors).await?;
        self.restore_genres(&mut tx, &data.genres).await?;
        self.restore_books(&mut tx, &data.books).await?;
        self.restore_book_authors(&mut tx, &data.book_authors)
            .await?;
        self.restore_readings(&mut tx, &data.readings).await?;
        self.restore_timeline_events(&mut tx, &data.timeline_events)
            .await?;
        self.restore_images(&mut tx, &data.images).await?;

        tx.commit().await.context("failed to commit transaction")?;

        Ok(())
    }

    /// Delete all book data, leaving auth tables intact.
    pub async fn reset(&self) -> anyhow::Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin transaction")?;

        let tables = [
            "entity_images",
            "readings",
            "book_authors",
            "books",
            "genres",
            "timeline_events",
            "authors",
            "stats_cache",
        ];

        for table in tables {
            let query = format!("DELETE FROM {table}");
            sqlx::query(&query)
                .execute(&mut *tx)
                .await
                .with_context(|| format!("failed to delete from {table}"))?;
        }

        tx.commit().await.context("failed to commit transaction")?;

        Ok(())
    }

    // --- Export methods ---

    async fn export_authors(
        &self,
        tx: &mut DatabaseTransaction<'_>,
    ) -> anyhow::Result<Vec<Author>> {
        let records = sqlx::query_as::<_, AuthorRecord>(
            "SELECT id, name, created_at FROM authors ORDER BY id",
        )
        .fetch_all(&mut **tx)
        .await
        .context("failed to export authors")?;

        Ok(records.into_iter().map(AuthorRecord::into_domain).collect())
    }

    async fn export_genres(&self, tx: &mut DatabaseTransaction<'_>) -> anyhow::Result<Vec<Genre>> {
        let records =
            sqlx::query_as::<_, GenreRecord>("SELECT id, name, created_at FROM genres ORDER BY id")
                .fetch_all(&mut **tx)
                .await
                .context("failed to export genres")?;

        Ok(records.into_iter().map(GenreRecord::into_domain).collect())
    }

    async fn export_books(&self, tx: &mut DatabaseTransaction<'_>) -> anyhow::Result<Vec<Book>> {
        let records = sqlx::query_as::<_, BookRecord>(
            "SELECT id, title, isbn, description, page_count, year_published, publisher, language, primary_genre_id, secondary_genre_id, created_at FROM books ORDER BY id",
        )
        .fetch_all(&mut **tx)
        .await
        .context("failed to export books")?;

        Ok(records.into_iter().map(BookRecord::into_domain).collect())
    }

    async fn export_book_authors(
        &self,
        tx: &mut DatabaseTransaction<'_>,
    ) -> anyhow::Result<Vec<BackupBookAuthor>> {
        let records = sqlx::query_as::<_, BookAuthorRecord>(
            "SELECT book_id, author_id, role FROM book_authors ORDER BY book_id, rowid",
        )
        .fetch_all(&mut **tx)
        .await
        .context("failed to export book_authors")?;

        Ok(records
            .into_iter()
            .map(|r| BackupBookAuthor {
                book_id: r.book_id,
                author_id: r.author_id,
                role: r.role,
            })
            .collect())
    }

    async fn export_readings(
        &self,
        tx: &mut DatabaseTransaction<'_>,
    ) -> anyhow::Result<Vec<Reading>> {
        let records = sqlx::query_as::<_, ReadingRecord>(
            "SELECT id, user_id, book_id, status, format, started_at, finished_at, rating, review, created_at, updated_at FROM readings ORDER BY id",
        )
        .fetch_all(&mut **tx)
        .await
        .context("failed to export readings")?;

        records
            .into_iter()
            .map(ReadingRecord::into_domain)
            .collect::<anyhow::Result<Vec<_>>>()
    }

    async fn export_timeline_events(
        &self,
        tx: &mut DatabaseTransaction<'_>,
    ) -> anyhow::Result<Vec<TimelineEvent>> {
        let records = sqlx::query_as::<_, TimelineEventRecord>(
            "SELECT id, entity_type, entity_id, action, occurred_at, title, details_json, genres_json, reading_data_json FROM timeline_events ORDER BY id",
        )
        .fetch_all(&mut **tx)
        .await
        .context("failed to export timeline events")?;

        records
            .into_iter()
            .map(TimelineEventRecord::into_domain)
            .collect::<anyhow::Result<Vec<_>>>()
    }

    async fn export_images(
        &self,
        tx: &mut DatabaseTransaction<'_>,
    ) -> anyhow::Result<Vec<BackupImage>> {
        let records = sqlx::query_as::<_, ImageRecord>(
            "SELECT entity_type, entity_id, content_type, image_data, thumbnail_data FROM entity_images ORDER BY entity_type, entity_id",
        )
        .fetch_all(&mut **tx)
        .await
        .context("failed to export images")?;

        Ok(records.into_iter().map(ImageRecord::into_backup).collect())
    }

    // --- Restore methods ---

    async fn verify_empty_database(&self) -> anyhow::Result<()> {
        let tables = [
            "authors",
            "genres",
            "books",
            "book_authors",
            "readings",
            "timeline_events",
            "entity_images",
        ];

        for table in tables {
            let query = format!("SELECT COUNT(*) as count FROM {table}");
            let row: (i64,) = sqlx::query_as(&query)
                .fetch_one(&self.pool)
                .await
                .with_context(|| format!("failed to check table {table}"))?;

            if row.0 > 0 {
                bail!(
                    "Cannot restore: table '{table}' is not empty ({} rows). Restore requires an empty database.",
                    row.0
                );
            }
        }

        Ok(())
    }

    async fn restore_authors(
        &self,
        tx: &mut DatabaseTransaction<'_>,
        authors: &[Author],
    ) -> anyhow::Result<()> {
        for author in authors {
            sqlx::query("INSERT INTO authors (id, name, created_at) VALUES (?, ?, ?)")
                .bind(i64::from(author.id))
                .bind(&author.name)
                .bind(author.created_at)
                .execute(&mut **tx)
                .await
                .context("failed to restore author")?;
        }

        Ok(())
    }

    async fn restore_genres(
        &self,
        tx: &mut DatabaseTransaction<'_>,
        genres: &[Genre],
    ) -> anyhow::Result<()> {
        for genre in genres {
            sqlx::query("INSERT INTO genres (id, name, created_at) VALUES (?, ?, ?)")
                .bind(i64::from(genre.id))
                .bind(&genre.name)
                .bind(genre.created_at)
                .execute(&mut **tx)
                .await
                .context("failed to restore genre")?;
        }

        Ok(())
    }

    async fn restore_books(
        &self,
        tx: &mut DatabaseTransaction<'_>,
        books: &[Book],
    ) -> anyhow::Result<()> {
        for book in books {
            sqlx::query(
                "INSERT INTO books (id, title, isbn, description, page_count, year_published, publisher, language, primary_genre_id, secondary_genre_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(i64::from(book.id))
            .bind(&book.title)
            .bind(book.isbn.as_deref())
            .bind(book.description.as_deref())
            .bind(book.page_count)
            .bind(book.year_published)
            .bind(book.publisher.as_deref())
            .bind(book.language.as_deref())
            .bind(book.primary_genre_id.map(i64::from))
            .bind(book.secondary_genre_id.map(i64::from))
            .bind(book.created_at)
            .execute(&mut **tx)
            .await
            .context("failed to restore book")?;
        }

        Ok(())
    }

    async fn restore_book_authors(
        &self,
        tx: &mut DatabaseTransaction<'_>,
        book_authors: &[BackupBookAuthor],
    ) -> anyhow::Result<()> {
        for ba in book_authors {
            sqlx::query("INSERT INTO book_authors (book_id, author_id, role) VALUES (?, ?, ?)")
                .bind(ba.book_id)
                .bind(ba.author_id)
                .bind(&ba.role)
                .execute(&mut **tx)
                .await
                .context("failed to restore book_author")?;
        }

        Ok(())
    }

    async fn restore_readings(
        &self,
        tx: &mut DatabaseTransaction<'_>,
        readings: &[Reading],
    ) -> anyhow::Result<()> {
        for reading in readings {
            sqlx::query(
                "INSERT INTO readings (id, user_id, book_id, status, format, started_at, finished_at, rating, review, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(i64::from(reading.id))
            .bind(i64::from(reading.user_id))
            .bind(i64::from(reading.book_id))
            .bind(reading.status.as_str())
            .bind(reading.format.map(|f| f.as_str().to_string()))
            .bind(reading.started_at)
            .bind(reading.finished_at)
            .bind(reading.rating)
            .bind(encode_quick_reviews(&reading.quick_reviews))
            .bind(reading.created_at)
            .bind(reading.updated_at)
            .execute(&mut **tx)
            .await
            .context("failed to restore reading")?;
        }

        Ok(())
    }

    async fn restore_timeline_events(
        &self,
        tx: &mut DatabaseTransaction<'_>,
        events: &[TimelineEvent],
    ) -> anyhow::Result<()> {
        for event in events {
            let details_json = to_string(&event.details)
                .context("failed to encode timeline event details for restore")?;

            let genres_json = to_string(&event.genres)
                .context("failed to encode timeline event genres for restore")?;

            let reading_data_json = event
                .reading_data
                .as_ref()
                .map(to_string)
                .transpose()
                .context("failed to encode timeline reading data for restore")?;

            sqlx::query(
                "INSERT INTO timeline_events (id, entity_type, entity_id, action, occurred_at, title, details_json, genres_json, reading_data_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(i64::from(event.id))
            .bind(&event.entity_type)
            .bind(event.entity_id)
            .bind(&event.action)
            .bind(event.occurred_at)
            .bind(&event.title)
            .bind(&details_json)
            .bind(&genres_json)
            .bind(reading_data_json.as_deref())
            .execute(&mut **tx)
            .await
            .context("failed to restore timeline event")?;
        }

        Ok(())
    }

    async fn restore_images(
        &self,
        tx: &mut DatabaseTransaction<'_>,
        images: &[BackupImage],
    ) -> anyhow::Result<()> {
        for image in images {
            sqlx::query(
                "INSERT INTO entity_images (entity_type, entity_id, content_type, image_data, thumbnail_data) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&image.entity_type)
            .bind(image.entity_id)
            .bind(&image.content_type)
            .bind(&image.image_data)
            .bind(&image.thumbnail_data)
            .execute(&mut **tx)
            .await
            .context("failed to restore image")?;
        }

        Ok(())
    }
}

// --- Record types for export queries ---

#[derive(sqlx::FromRow)]
struct AuthorRecord {
    id: i64,
    name: String,
    created_at: DateTime<Utc>,
}

impl AuthorRecord {
    fn into_domain(self) -> Author {
        Author {
            id: AuthorId::from(self.id),
            name: self.name,
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct GenreRecord {
    id: i64,
    name: String,
    created_at: DateTime<Utc>,
}

impl GenreRecord {
    fn into_domain(self) -> Genre {
        Genre {
            id: GenreId::from(self.id),
            name: self.name,
            created_at: self.created_at,
        }
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

impl BookRecord {
    fn into_domain(self) -> Book {
        Book {
            id: BookId::from(self.id),
            title: self.title,
            isbn: self.isbn,
            description: self.description,
            page_count: self.page_count,
            year_published: self.year_published,
            publisher: self.publisher,
            language: self.language,
            primary_genre_id: self.primary_genre_id.map(GenreId::from),
            secondary_genre_id: self.secondary_genre_id.map(GenreId::from),
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct BookAuthorRecord {
    book_id: i64,
    author_id: i64,
    role: String,
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

impl ReadingRecord {
    fn into_domain(self) -> anyhow::Result<Reading> {
        let status = ReadingStatus::from_str(&self.status)
            .map_err(|()| anyhow::anyhow!("invalid reading status: {}", self.status))?;
        let format = self
            .format
            .as_deref()
            .map(|s| {
                ReadingFormat::from_str(s)
                    .map_err(|()| anyhow::anyhow!("invalid reading format: {s}"))
            })
            .transpose()?;

        Ok(Reading {
            id: ReadingId::from(self.id),
            user_id: UserId::from(self.user_id),
            book_id: BookId::from(self.book_id),
            status,
            format,
            started_at: self.started_at,
            finished_at: self.finished_at,
            rating: self.rating,
            quick_reviews: crate::infrastructure::repositories::books::readings::SqlReadingRepository::decode_quick_reviews(self.review),
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
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
    fn into_domain(self) -> anyhow::Result<TimelineEvent> {
        let details = decode_json_vec(self.details_json, "timeline event details")?;
        let genres = decode_json_vec(self.genres_json, "timeline genres")?;
        let reading_data = decode_json_opt(self.reading_data_json, "timeline reading data")?;

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

#[derive(sqlx::FromRow)]
struct ImageRecord {
    entity_type: String,
    entity_id: i64,
    content_type: String,
    image_data: Vec<u8>,
    thumbnail_data: Vec<u8>,
}

impl ImageRecord {
    fn into_backup(self) -> BackupImage {
        BackupImage {
            entity_type: self.entity_type,
            entity_id: self.entity_id,
            content_type: self.content_type,
            image_data: self.image_data,
            thumbnail_data: self.thumbnail_data,
        }
    }
}
