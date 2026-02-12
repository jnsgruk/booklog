use std::sync::Arc;

use booklog::application::services::{AuthorService, BookService};
use booklog::domain::authors::{Author, AuthorSortKey, NewAuthor};
use booklog::domain::book_items::{AuthorRole, Book, BookAuthor, BookSortKey, NewBook};
use booklog::domain::genres::{Genre, NewGenre};
use booklog::domain::ids::{AuthorId, BookId, GenreId, UserId};
use booklog::domain::listing::{ListRequest, PageSize};
use booklog::domain::readings::{
    NewReading, Reading, ReadingFilter, ReadingFormat, ReadingSortKey, ReadingStatus,
};
use booklog::domain::repositories::{
    AuthorRepository, BookRepository, GenreRepository, ReadingRepository, TimelineEventRepository,
};
use booklog::domain::timeline::TimelineEvent;
use booklog::infrastructure::backup::{BackupData, BackupService};
use booklog::infrastructure::database::{Database, DatabasePool};
use booklog::infrastructure::repositories::authors::SqlAuthorRepository;
use booklog::infrastructure::repositories::book_repos::SqlBookRepository;
use booklog::infrastructure::repositories::genres::SqlGenreRepository;
use booklog::infrastructure::repositories::readings::SqlReadingRepository;
use booklog::infrastructure::repositories::timeline_events::SqlTimelineEventRepository;

use super::helpers::{create_default_author, spawn_app, spawn_app_with_auth};

struct TestDb {
    pool: DatabasePool,
    author_repo: Arc<dyn AuthorRepository>,
    genre_repo: Arc<dyn GenreRepository>,
    book_repo: Arc<dyn BookRepository>,
    reading_repo: Arc<dyn ReadingRepository>,
    timeline_repo: Arc<dyn TimelineEventRepository>,
    backup_service: BackupService,
    author_service: AuthorService,
    book_service: BookService,
}

async fn create_test_db() -> TestDb {
    let database = Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory database");

    let pool = database.clone_pool();

    let author_repo: Arc<dyn AuthorRepository> = Arc::new(SqlAuthorRepository::new(pool.clone()));
    let genre_repo: Arc<dyn GenreRepository> = Arc::new(SqlGenreRepository::new(pool.clone()));
    let book_repo: Arc<dyn BookRepository> = Arc::new(SqlBookRepository::new(pool.clone()));
    let reading_repo: Arc<dyn ReadingRepository> =
        Arc::new(SqlReadingRepository::new(pool.clone()));
    let timeline_repo: Arc<dyn TimelineEventRepository> =
        Arc::new(SqlTimelineEventRepository::new(pool.clone()));

    let author_service = AuthorService::new(Arc::clone(&author_repo), Arc::clone(&timeline_repo));
    let book_service = BookService::new(
        Arc::clone(&book_repo),
        Arc::clone(&author_repo),
        Arc::clone(&genre_repo),
        Arc::clone(&timeline_repo),
    );

    TestDb {
        pool: pool.clone(),
        author_repo,
        genre_repo,
        book_repo,
        reading_repo,
        timeline_repo,
        backup_service: BackupService::new(pool),
        author_service,
        book_service,
    }
}

fn list_all_request<K: booklog::domain::listing::SortKey>() -> ListRequest<K> {
    ListRequest::new(
        1,
        PageSize::All,
        K::default(),
        K::default().default_direction(),
    )
}

async fn list_all_authors(repo: &dyn AuthorRepository) -> Vec<Author> {
    repo.list(&list_all_request::<AuthorSortKey>(), None)
        .await
        .expect("failed to list authors")
        .items
}

async fn list_all_books(repo: &dyn BookRepository) -> Vec<Book> {
    let page = repo
        .list(&list_all_request::<BookSortKey>(), None)
        .await
        .expect("failed to list books");
    page.items.into_iter().map(|bwa| bwa.book).collect()
}

async fn list_all_readings(repo: &dyn ReadingRepository) -> Vec<Reading> {
    let page = repo
        .list(
            ReadingFilter::all(),
            &list_all_request::<ReadingSortKey>(),
            None,
        )
        .await
        .expect("failed to list readings");
    page.items.into_iter().map(|rwb| rwb.reading).collect()
}

async fn list_all_timeline_events(repo: &dyn TimelineEventRepository) -> Vec<TimelineEvent> {
    repo.list_all()
        .await
        .expect("failed to list timeline events")
}

async fn insert_test_image(pool: &DatabasePool, entity_type: &str, entity_id: i64) {
    sqlx::query(
        "INSERT INTO entity_images (entity_type, entity_id, content_type, image_data, thumbnail_data) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind("image/png")
    .bind(b"fake-image-data".as_slice())
    .bind(b"fake-thumb-data".as_slice())
    .execute(pool)
    .await
    .expect("failed to insert test image");
}

async fn count_images(pool: &DatabasePool) -> i64 {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM entity_images")
        .fetch_one(pool)
        .await
        .expect("failed to count images");
    row.0
}

/// Insert a test user into the database and return their ID.
async fn insert_test_user(pool: &DatabasePool) -> UserId {
    sqlx::query("INSERT INTO users (username, uuid, created_at) VALUES (?, ?, datetime('now'))")
        .bind("testuser")
        .bind(uuid::Uuid::new_v4().to_string())
        .execute(pool)
        .await
        .expect("failed to create test user");

    let user_id: i64 = sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE username = ?")
        .bind("testuser")
        .fetch_one(pool)
        .await
        .expect("failed to fetch test user id");

    UserId::new(user_id)
}

/// Populate a database with representative test data and return the key entities.
async fn populate_test_data(db: &TestDb) -> (Author, Genre, Genre, Book, Reading) {
    let user_id = insert_test_user(&db.pool).await;

    // Create author (via service to generate timeline event)
    let author = db
        .author_service
        .create(
            NewAuthor {
                name: "Ursula K. Le Guin".to_string(),
                created_at: None,
            },
            user_id,
        )
        .await
        .expect("failed to create author");

    // Create genres
    let sci_fi = db
        .genre_repo
        .insert(NewGenre {
            name: "Science Fiction".to_string(),
            created_at: None,
        })
        .await
        .expect("failed to create genre");

    let fantasy = db
        .genre_repo
        .insert(NewGenre {
            name: "Fantasy".to_string(),
            created_at: None,
        })
        .await
        .expect("failed to create genre");

    // Create book (via service to generate timeline event)
    let book = db
        .book_service
        .create(
            NewBook {
                title: "The Left Hand of Darkness".to_string(),
                authors: vec![BookAuthor {
                    author_id: author.id,
                    role: AuthorRole::default(),
                }],
                isbn: Some("978-0441478125".to_string()),
                description: Some("A science fiction novel".to_string()),
                page_count: Some(304),
                year_published: Some(1969),
                publisher: Some("Ace Books".to_string()),
                language: Some("English".to_string()),
                primary_genre_id: Some(sci_fi.id),
                secondary_genre_id: Some(fantasy.id),
                created_at: None,
            },
            user_id,
        )
        .await
        .expect("failed to create book");

    // Create reading
    let reading = db
        .reading_repo
        .insert(NewReading {
            user_id,
            book_id: book.id,
            status: ReadingStatus::Reading,
            format: Some(ReadingFormat::Physical),
            started_at: Some(chrono::NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()),
            finished_at: None,
            rating: None,
            quick_reviews: Vec::new(),
            created_at: None,
        })
        .await
        .expect("failed to create reading");

    (author, sci_fi, fantasy, book, reading)
}

#[tokio::test]
async fn backup_and_restore_round_trip() {
    // 1. Create source database and populate with test data
    let source = create_test_db().await;
    let (author, _sci_fi, _fantasy, book, reading) = populate_test_data(&source).await;

    // Insert a test image for the author
    insert_test_image(&source.pool, "author", i64::from(author.id)).await;

    // Verify timeline events were created (author + book inserts create them)
    let source_timeline = list_all_timeline_events(source.timeline_repo.as_ref()).await;
    assert!(
        source_timeline.len() >= 2,
        "expected at least 2 timeline events from author+book creation"
    );

    // 2. Export backup
    let backup_data = source
        .backup_service
        .export()
        .await
        .expect("failed to export backup");

    assert_eq!(backup_data.version, 3);
    assert_eq!(backup_data.authors.len(), 1);
    assert_eq!(backup_data.genres.len(), 2);
    assert_eq!(backup_data.books.len(), 1);
    assert_eq!(backup_data.book_authors.len(), 1);
    assert_eq!(backup_data.readings.len(), 1);
    assert_eq!(backup_data.timeline_events.len(), source_timeline.len());
    assert_eq!(backup_data.images.len(), 1);
    assert_eq!(backup_data.images[0].entity_type, "author");
    assert_eq!(backup_data.images[0].content_type, "image/png");

    // 3. Serialize to JSON and deserialize back (verify serde round-trip)
    let json = serde_json::to_string_pretty(&backup_data).expect("failed to serialize backup");
    let restored_data: BackupData =
        serde_json::from_str(&json).expect("failed to deserialize backup");

    assert_eq!(restored_data.version, 3);
    assert_eq!(restored_data.authors.len(), 1);
    assert_eq!(restored_data.genres.len(), 2);
    assert_eq!(restored_data.books.len(), 1);
    assert_eq!(restored_data.book_authors.len(), 1);
    assert_eq!(restored_data.readings.len(), 1);
    assert_eq!(restored_data.images.len(), 1);

    // 4. Restore to a fresh database
    let target = create_test_db().await;
    // The reading has a user_id FK, so we need a user in the target DB before restoring.
    insert_test_user(&target.pool).await;
    target
        .backup_service
        .restore(restored_data)
        .await
        .expect("failed to restore backup");

    // 5. Verify all data matches

    // Authors
    let target_authors = list_all_authors(target.author_repo.as_ref()).await;
    assert_eq!(target_authors.len(), 1);
    let restored_author = &target_authors[0];
    assert_eq!(restored_author.id, author.id);
    assert_eq!(restored_author.name, author.name);
    assert_eq!(restored_author.created_at, author.created_at);

    // Books
    let target_books = list_all_books(target.book_repo.as_ref()).await;
    assert_eq!(target_books.len(), 1);
    let restored_book = &target_books[0];
    assert_eq!(restored_book.id, book.id);
    assert_eq!(restored_book.title, book.title);
    assert_eq!(restored_book.isbn, book.isbn);
    assert_eq!(restored_book.description, book.description);
    assert_eq!(restored_book.page_count, book.page_count);
    assert_eq!(restored_book.year_published, book.year_published);
    assert_eq!(restored_book.publisher, book.publisher);
    assert_eq!(restored_book.language, book.language);
    assert_eq!(restored_book.primary_genre_id, book.primary_genre_id);
    assert_eq!(restored_book.secondary_genre_id, book.secondary_genre_id);

    // Readings
    let target_readings = list_all_readings(target.reading_repo.as_ref()).await;
    assert_eq!(target_readings.len(), 1);
    let restored_reading = &target_readings[0];
    assert_eq!(restored_reading.id, reading.id);
    assert_eq!(restored_reading.book_id, reading.book_id);
    assert_eq!(restored_reading.status, reading.status);
    assert_eq!(restored_reading.started_at, reading.started_at);
    assert_eq!(restored_reading.finished_at, reading.finished_at);
    assert_eq!(restored_reading.rating, reading.rating);
    assert_eq!(restored_reading.quick_reviews, reading.quick_reviews);

    // Timeline events
    let target_timeline = list_all_timeline_events(target.timeline_repo.as_ref()).await;
    assert_eq!(target_timeline.len(), source_timeline.len());
    for (source_event, target_event) in source_timeline.iter().zip(target_timeline.iter()) {
        assert_eq!(target_event.id, source_event.id);
        assert_eq!(target_event.entity_type, source_event.entity_type);
        assert_eq!(target_event.entity_id, source_event.entity_id);
        assert_eq!(target_event.action, source_event.action);
        assert_eq!(target_event.title, source_event.title);
        assert_eq!(target_event.details.len(), source_event.details.len());
        assert_eq!(target_event.genres, source_event.genres);
    }

    // Images
    assert_eq!(count_images(&target.pool).await, 1);
    let target_backup = target
        .backup_service
        .export()
        .await
        .expect("failed to re-export");
    assert_eq!(target_backup.images.len(), 1);
    assert_eq!(target_backup.images[0].entity_type, "author");
    assert_eq!(target_backup.images[0].image_data, b"fake-image-data");
    assert_eq!(target_backup.images[0].thumbnail_data, b"fake-thumb-data");
}

#[tokio::test]
async fn restore_to_non_empty_database_fails() {
    let db = create_test_db().await;

    // Add an author to make the database non-empty
    db.author_repo
        .insert(NewAuthor {
            name: "Existing Author".to_string(),
            created_at: None,
        })
        .await
        .expect("failed to create author");

    // Create a minimal backup
    let backup_data = BackupData {
        version: 3,
        created_at: chrono::Utc::now(),
        authors: vec![],
        genres: vec![],
        books: vec![],
        book_authors: vec![],
        readings: vec![],
        timeline_events: vec![],
        images: vec![],
    };

    // Restore should fail because the database is not empty
    let result = db.backup_service.restore(backup_data).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not empty"),
        "expected 'not empty' error, got: {err_msg}"
    );
}

#[tokio::test]
async fn backup_empty_database() {
    let db = create_test_db().await;

    let backup_data = db
        .backup_service
        .export()
        .await
        .expect("failed to export empty database");

    assert_eq!(backup_data.version, 3);
    assert!(backup_data.authors.is_empty());
    assert!(backup_data.genres.is_empty());
    assert!(backup_data.books.is_empty());
    assert!(backup_data.book_authors.is_empty());
    assert!(backup_data.readings.is_empty());
    assert!(backup_data.timeline_events.is_empty());
    assert!(backup_data.images.is_empty());

    // Should serialize to valid JSON
    let json = serde_json::to_string_pretty(&backup_data).expect("failed to serialize");
    let parsed: BackupData = serde_json::from_str(&json).expect("failed to deserialize");
    assert_eq!(parsed.version, 3);
}

// --- API-level tests ---

#[tokio::test]
async fn backup_export_requires_auth() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.api_url("/backup"))
        .send()
        .await
        .expect("failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn backup_export_returns_data() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    // Create some data first
    create_default_author(&app).await;

    let response = client
        .get(app.api_url("/backup"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let data: BackupData = response.json().await.expect("failed to parse backup data");
    assert_eq!(data.version, 3);
    assert_eq!(data.authors.len(), 1);
    assert_eq!(data.authors[0].name, "Test Author");
}

#[tokio::test]
async fn backup_restore_requires_auth() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let backup_data = BackupData {
        version: 3,
        created_at: chrono::Utc::now(),
        authors: vec![],
        genres: vec![],
        books: vec![],
        book_authors: vec![],
        readings: vec![],
        timeline_events: vec![],
        images: vec![],
    };

    let response = client
        .post(app.api_url("/backup/restore"))
        .json(&backup_data)
        .send()
        .await
        .expect("failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn backup_restore_non_empty_db_returns_conflict() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    // Create data to make the database non-empty
    create_default_author(&app).await;

    let backup_data = BackupData {
        version: 3,
        created_at: chrono::Utc::now(),
        authors: vec![],
        genres: vec![],
        books: vec![],
        book_authors: vec![],
        readings: vec![],
        timeline_events: vec![],
        images: vec![],
    };

    let response = client
        .post(app.api_url("/backup/restore"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&backup_data)
        .send()
        .await
        .expect("failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
}

#[tokio::test]
async fn backup_round_trip_via_api() {
    // 1. Create source app with data
    let source = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    create_default_author(&source).await;

    // 2. Export via API
    let response = client
        .get(source.api_url("/backup"))
        .bearer_auth(source.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("failed to export backup");

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let backup_data: BackupData = response.json().await.expect("failed to parse backup");
    assert_eq!(backup_data.authors.len(), 1);

    // 3. Restore into a fresh app
    let target = spawn_app_with_auth().await;

    let response = client
        .post(target.api_url("/backup/restore"))
        .bearer_auth(target.auth_token.as_ref().unwrap())
        .json(&backup_data)
        .send()
        .await
        .expect("failed to restore backup");

    assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);

    // 4. Verify data was restored by listing authors
    let response = client
        .get(target.api_url("/authors"))
        .send()
        .await
        .expect("failed to list authors");

    let authors: Vec<Author> = response.json().await.expect("failed to parse authors");
    assert_eq!(authors.len(), 1);
    assert_eq!(authors[0].name, "Test Author");
}

// --- Reset tests ---

#[tokio::test]
async fn reset_requires_auth() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .post(app.api_url("/backup/reset"))
        .send()
        .await
        .expect("failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn reset_clears_all_data() {
    let db = create_test_db().await;
    let (author, ..) = populate_test_data(&db).await;

    // Insert an image
    insert_test_image(&db.pool, "author", i64::from(author.id)).await;

    // Verify genres exist before reset
    let genre_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM genres")
        .fetch_one(&db.pool)
        .await
        .expect("failed to count genres");
    assert!(genre_count.0 > 0);

    // Verify data exists before reset
    assert!(!list_all_authors(db.author_repo.as_ref()).await.is_empty());
    assert!(!list_all_books(db.book_repo.as_ref()).await.is_empty());
    assert!(!list_all_readings(db.reading_repo.as_ref()).await.is_empty());
    assert!(
        !list_all_timeline_events(db.timeline_repo.as_ref())
            .await
            .is_empty()
    );
    assert_eq!(count_images(&db.pool).await, 1);

    // Reset
    db.backup_service
        .reset()
        .await
        .expect("failed to reset database");

    // Verify all tables are empty
    assert!(list_all_authors(db.author_repo.as_ref()).await.is_empty());
    assert!(list_all_books(db.book_repo.as_ref()).await.is_empty());
    assert!(list_all_readings(db.reading_repo.as_ref()).await.is_empty());
    assert!(
        list_all_timeline_events(db.timeline_repo.as_ref())
            .await
            .is_empty()
    );
    assert_eq!(count_images(&db.pool).await, 0);
    let genre_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM genres")
        .fetch_one(&db.pool)
        .await
        .expect("failed to count genres");
    assert_eq!(genre_count.0, 0);
}

#[tokio::test]
async fn reset_then_restore_succeeds() {
    let db = create_test_db().await;
    populate_test_data(&db).await;

    // Export before reset
    let backup = db.backup_service.export().await.expect("failed to export");
    assert_eq!(backup.authors.len(), 1);
    assert_eq!(backup.readings.len(), 1);

    // Reset
    db.backup_service
        .reset()
        .await
        .expect("failed to reset database");

    // Restore should succeed (database is now empty)
    db.backup_service
        .restore(backup)
        .await
        .expect("failed to restore after reset");

    // Verify data is back
    assert_eq!(list_all_authors(db.author_repo.as_ref()).await.len(), 1);
    assert_eq!(list_all_books(db.book_repo.as_ref()).await.len(), 1);
    assert_eq!(list_all_readings(db.reading_repo.as_ref()).await.len(), 1);
}

// --- Error path tests ---

#[tokio::test]
async fn restore_rejects_malformed_json() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let response = client
        .post(app.api_url("/backup/restore"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("content-type", "application/json")
        .body("this is not valid json {{{")
        .send()
        .await
        .expect("failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn restore_rejects_json_missing_required_fields() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    // Valid JSON but missing the required "version", "created_at", etc.
    let response = client
        .post(app.api_url("/backup/restore"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&serde_json::json!({"authors": []}))
        .send()
        .await
        .expect("failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn restore_with_fk_violation_returns_error() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    // A book referencing a genre ID that doesn't exist in the backup
    let backup = serde_json::json!({
        "version": 3,
        "created_at": "2026-01-01T00:00:00Z",
        "authors": [],
        "genres": [],
        "books": [{
            "id": 1,
            "title": "Orphan Book",
            "isbn": null,
            "description": null,
            "page_count": null,
            "year_published": null,
            "publisher": null,
            "language": null,
            "primary_genre_id": 999,
            "secondary_genre_id": null,
            "created_at": "2026-01-01T00:00:00Z"
        }],
        "book_authors": [],
        "readings": [],
        "timeline_events": [],
        "images": []
    });

    let response = client
        .post(app.api_url("/backup/restore"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&backup)
        .send()
        .await
        .expect("failed to send request");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn restore_with_fk_violation_does_not_partially_commit() {
    let db = create_test_db().await;

    // A backup with a valid author but a book referencing a non-existent genre.
    // The transaction should roll back, leaving the authors table empty.
    let backup = BackupData {
        version: 3,
        created_at: chrono::Utc::now(),
        authors: vec![Author {
            id: AuthorId::from(1i64),
            name: "Rollback Author".to_string(),
            created_at: chrono::Utc::now(),
        }],
        genres: vec![],
        books: vec![Book {
            id: BookId::from(1i64),
            title: "Bad FK Book".to_string(),
            isbn: None,
            description: None,
            page_count: None,
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: Some(GenreId::from(999i64)),
            secondary_genre_id: None,
            created_at: chrono::Utc::now(),
        }],
        book_authors: vec![],
        readings: vec![],
        timeline_events: vec![],
        images: vec![],
    };

    let result = db.backup_service.restore(backup).await;
    assert!(result.is_err(), "restore should fail on FK violation");

    // The author should NOT have been committed since the whole transaction rolled back
    let authors = list_all_authors(db.author_repo.as_ref()).await;
    assert!(
        authors.is_empty(),
        "transaction should have rolled back, but found {} authors",
        authors.len()
    );
}
