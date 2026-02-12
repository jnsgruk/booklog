use reqwest::Client;

use crate::helpers::{
    assert_full_page, create_author_with_name, create_default_author, create_default_book,
    create_entity, create_genre_with_name, create_session, spawn_app, spawn_app_with_auth,
};

#[tokio::test]
async fn stats_page_returns_200_with_empty_database() {
    let app = spawn_app().await;
    let client = Client::new();

    let response = client
        .get(app.page_url("/stats"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
    assert!(body.contains("Stats"), "Page should contain title");
}

#[tokio::test]
async fn stats_page_returns_200_with_data() {
    let app = spawn_app_with_auth().await;

    let author = create_default_author(&app).await;
    let _book = create_default_book(&app, author.id).await;

    let client = Client::new();
    let response = client
        .get(app.page_url("/stats"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
}

#[tokio::test]
async fn recompute_stats_requires_authentication() {
    let app = spawn_app().await;
    let client = Client::new();

    let response = client
        .post(app.api_url("/stats/recompute"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn recompute_stats_returns_json() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .post(app.api_url("/stats/recompute"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(
        body.get("computed_at").is_some(),
        "Response should contain computed_at"
    );
}

#[tokio::test]
async fn recompute_stats_reflects_created_data() {
    let app = spawn_app_with_auth().await;

    let author = create_default_author(&app).await;
    let _book = create_default_book(&app, author.id).await;

    let client = Client::new();
    let response = client
        .post(app.api_url("/stats/recompute"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(
        body.get("computed_at").is_some(),
        "Response should contain computed_at"
    );
}

#[tokio::test]
async fn stats_page_loads_after_recompute() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    // Populate the cache
    let recompute_response = client
        .post(app.api_url("/stats/recompute"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to recompute");
    assert_eq!(recompute_response.status(), 200);

    // Load the page -- should read from cache
    let response = client
        .get(app.page_url("/stats"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
}

#[tokio::test]
async fn recompute_stats_includes_extended_fields() {
    let app = spawn_app_with_auth().await;

    // Create an author
    let author = create_entity::<_, booklog::domain::authors::Author>(
        &app,
        "/authors",
        &booklog::domain::authors::NewAuthor {
            name: "Jane Austen".to_string(),
            created_at: None,
        },
    )
    .await;

    // Create genres
    let fiction = create_genre_with_name(&app, "Fiction").await;
    let romance = create_genre_with_name(&app, "Romance").await;

    // Create a book with page count, genres, and year
    let book = create_entity::<_, booklog::domain::book_items::Book>(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "Pride and Prejudice".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author.id,
                role: booklog::domain::book_items::AuthorRole::Author,
            }],
            isbn: None,
            description: None,
            page_count: Some(432),
            year_published: Some(1813),
            publisher: Some("T. Egerton".to_string()),
            language: Some("English".to_string()),
            primary_genre_id: Some(fiction.id),
            secondary_genre_id: Some(romance.id),
            created_at: None,
        },
    )
    .await;

    // Create a completed reading with dates, rating, and format
    let _reading = create_entity::<_, booklog::domain::readings::Reading>(
        &app,
        "/readings",
        &booklog::domain::readings::NewReading {
            user_id: booklog::domain::ids::UserId::new(1),
            book_id: book.id,
            status: booklog::domain::readings::ReadingStatus::Read,
            format: Some(booklog::domain::readings::ReadingFormat::Physical),
            started_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            finished_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()),
            rating: Some(4.0),
            quick_reviews: Vec::new(),
            created_at: None,
        },
    )
    .await;

    let client = Client::new();
    let response = client
        .post(app.api_url("/stats/recompute"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    let book_summary = &body["book_summary"];
    let reading = &body["reading"];

    // Book summary extended fields
    assert_eq!(book_summary["total_books"], 1);
    assert_eq!(book_summary["total_authors"], 1);
    assert!(
        !book_summary["page_count_distribution"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        !book_summary["year_published_distribution"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!book_summary["top_authors"].as_array().unwrap().is_empty());

    let longest = book_summary["longest_book"].as_array().unwrap();
    assert_eq!(longest[0], "Pride and Prejudice");
    assert_eq!(longest[1], 432);

    let shortest = book_summary["shortest_book"].as_array().unwrap();
    assert_eq!(shortest[0], "Pride and Prejudice");
    assert_eq!(shortest[1], 432);

    // Reading extended fields
    assert_eq!(reading["books_all_time"], 1);
    assert_eq!(reading["books_abandoned"], 0);
    assert!(reading["average_days_to_finish"].as_f64().unwrap() > 0.0);
    assert!(
        !reading["rating_distribution"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(reading["monthly_books"].as_array().unwrap().len(), 12);
    assert_eq!(reading["monthly_pages"].as_array().unwrap().len(), 12);
    assert!(!reading["pace_distribution"].as_array().unwrap().is_empty());
    assert!(!reading["format_counts"].as_array().unwrap().is_empty());
    assert!(reading["max_rating_count"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn stats_page_shows_new_sections_with_data() {
    let app = spawn_app_with_auth().await;

    // Create data
    let author = create_default_author(&app).await;
    let fiction = create_genre_with_name(&app, "Fiction").await;
    let book = create_entity::<_, booklog::domain::book_items::Book>(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "Test Book".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author.id,
                role: booklog::domain::book_items::AuthorRole::Author,
            }],
            isbn: None,
            description: None,
            page_count: Some(300),
            year_published: Some(2020),
            publisher: None,
            language: None,
            primary_genre_id: Some(fiction.id),
            secondary_genre_id: None,
            created_at: None,
        },
    )
    .await;

    let _reading = create_entity::<_, booklog::domain::readings::Reading>(
        &app,
        "/readings",
        &booklog::domain::readings::NewReading {
            user_id: booklog::domain::ids::UserId::new(1),
            book_id: book.id,
            status: booklog::domain::readings::ReadingStatus::Read,
            format: Some(booklog::domain::readings::ReadingFormat::Physical),
            started_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            finished_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()),
            rating: Some(5.0),
            quick_reviews: Vec::new(),
            created_at: None,
        },
    )
    .await;

    // Force recompute
    let client = Client::new();
    let recompute = client
        .post(app.api_url("/stats/recompute"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(recompute.status(), 200);

    // Load stats page with session cookie (stats are per-user)
    let session_token = create_session(&app).await;
    let response = client
        .get(app.page_url("/stats"))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to load stats page");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
    assert!(body.contains("Reading Summary"));
    assert!(body.contains("Ratings"));
    assert!(body.contains("genres"));
    assert!(body.contains("Page Count"));
    assert!(body.contains("Reading Pace"));
    assert!(body.contains("Top Authors"));
    assert!(body.contains("Format"));
    assert!(!body.contains("Publication Decades"));
    assert!(body.contains("Book Records"));
}

#[tokio::test]
async fn stats_page_with_year_filter_returns_200() {
    let app = spawn_app_with_auth().await;

    // Create data with a reading finished in 2026
    let author = create_default_author(&app).await;
    let book = create_entity::<_, booklog::domain::book_items::Book>(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "Year Test Book".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author.id,
                role: booklog::domain::book_items::AuthorRole::Author,
            }],
            isbn: None,
            description: None,
            page_count: Some(250),
            year_published: Some(2020),
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            created_at: None,
        },
    )
    .await;

    let _reading = create_entity::<_, booklog::domain::readings::Reading>(
        &app,
        "/readings",
        &booklog::domain::readings::NewReading {
            user_id: booklog::domain::ids::UserId::new(1),
            book_id: book.id,
            status: booklog::domain::readings::ReadingStatus::Read,
            format: Some(booklog::domain::readings::ReadingFormat::Physical),
            started_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            finished_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()),
            rating: Some(4.5),
            quick_reviews: Vec::new(),
            created_at: None,
        },
    )
    .await;

    let client = Client::new();
    let session_token = create_session(&app).await;

    let response = client
        .get(app.page_url("/stats?year=2026"))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to load stats page");

    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
    assert!(body.contains("Reading Summary"));
    assert!(!body.contains("In Progress"));
    assert!(!body.contains("Want to Read"));
}

#[tokio::test]
async fn stats_page_with_year_filter_shows_empty_for_no_data() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();
    let session_token = create_session(&app).await;

    let response = client
        .get(app.page_url("/stats?year=2020"))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to load stats page");

    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("Failed to read body");
    assert!(body.contains("No stats for this year"));
}

#[tokio::test]
async fn stats_page_shows_year_tabs_when_data_exists() {
    let app = spawn_app_with_auth().await;

    let author = create_default_author(&app).await;
    let book = create_entity::<_, booklog::domain::book_items::Book>(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "Tab Test Book".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author.id,
                role: booklog::domain::book_items::AuthorRole::Author,
            }],
            isbn: None,
            description: None,
            page_count: Some(200),
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            created_at: None,
        },
    )
    .await;

    let _reading = create_entity::<_, booklog::domain::readings::Reading>(
        &app,
        "/readings",
        &booklog::domain::readings::NewReading {
            user_id: booklog::domain::ids::UserId::new(1),
            book_id: book.id,
            status: booklog::domain::readings::ReadingStatus::Read,
            format: None,
            started_at: None,
            finished_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
            rating: None,
            quick_reviews: Vec::new(),
            created_at: None,
        },
    )
    .await;

    let client = Client::new();
    let session_token = create_session(&app).await;

    let response = client
        .get(app.page_url("/stats"))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to load stats page");

    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("Failed to read body");
    assert!(body.contains("All Time"));
    assert!(body.contains("2026"));
}

#[tokio::test]
async fn stats_page_datastar_fragment_returns_content_only() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();
    let session_token = create_session(&app).await;

    let response = client
        .get(app.page_url("/stats?year=all"))
        .header("Cookie", format!("booklog_session={session_token}"))
        .header("datastar-request", "true")
        .send()
        .await
        .expect("Failed to load stats fragment");

    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("Failed to read body");
    // Should NOT contain full page wrapper
    assert!(!body.contains("<!DOCTYPE"));
    assert!(!body.contains("<html"));
}

#[tokio::test]
async fn stats_with_books_missing_page_count_has_no_extremes() {
    let app = spawn_app_with_auth().await;

    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await; // page_count: None

    // Create a completed reading so the book appears in stats
    let _reading = create_entity::<_, booklog::domain::readings::Reading>(
        &app,
        "/readings",
        &booklog::domain::readings::NewReading {
            user_id: booklog::domain::ids::UserId::new(1),
            book_id: book.id,
            status: booklog::domain::readings::ReadingStatus::Read,
            format: None,
            started_at: None,
            finished_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
            rating: None,
            quick_reviews: Vec::new(),
            created_at: None,
        },
    )
    .await;

    let client = Client::new();
    let response = client
        .post(app.api_url("/stats/recompute"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let book_summary = &body["book_summary"];

    assert_eq!(book_summary["total_books"], 1);
    assert!(book_summary["longest_book"].is_null());
    assert!(book_summary["shortest_book"].is_null());
    assert!(
        book_summary["page_count_distribution"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn stats_with_books_missing_genres_has_empty_genre_counts() {
    let app = spawn_app_with_auth().await;

    let author = create_default_author(&app).await;
    // Book without genres
    let book = create_entity::<_, booklog::domain::book_items::Book>(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "No Genre Book".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author.id,
                role: booklog::domain::book_items::AuthorRole::Author,
            }],
            isbn: None,
            description: None,
            page_count: Some(200),
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            created_at: None,
        },
    )
    .await;

    let _reading = create_entity::<_, booklog::domain::readings::Reading>(
        &app,
        "/readings",
        &booklog::domain::readings::NewReading {
            user_id: booklog::domain::ids::UserId::new(1),
            book_id: book.id,
            status: booklog::domain::readings::ReadingStatus::Read,
            format: None,
            started_at: None,
            finished_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
            rating: None,
            quick_reviews: Vec::new(),
            created_at: None,
        },
    )
    .await;

    let client = Client::new();
    let response = client
        .post(app.api_url("/stats/recompute"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let book_summary = &body["book_summary"];

    assert_eq!(book_summary["total_books"], 1);
    assert_eq!(book_summary["unique_genres"], 0);
    assert!(book_summary["top_genre"].is_null());
    assert!(book_summary["genre_counts"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn stats_counts_abandoned_readings() {
    let app = spawn_app_with_auth().await;

    let author = create_default_author(&app).await;
    let book1 = create_entity::<_, booklog::domain::book_items::Book>(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "Abandoned Book".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author.id,
                role: booklog::domain::book_items::AuthorRole::Author,
            }],
            isbn: None,
            description: None,
            page_count: None,
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            created_at: None,
        },
    )
    .await;

    let book2 = create_entity::<_, booklog::domain::book_items::Book>(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "Finished Book".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author.id,
                role: booklog::domain::book_items::AuthorRole::Author,
            }],
            isbn: None,
            description: None,
            page_count: None,
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            created_at: None,
        },
    )
    .await;

    // One abandoned reading
    let _abandoned = create_entity::<_, booklog::domain::readings::Reading>(
        &app,
        "/readings",
        &booklog::domain::readings::NewReading {
            user_id: booklog::domain::ids::UserId::new(1),
            book_id: book1.id,
            status: booklog::domain::readings::ReadingStatus::Abandoned,
            format: None,
            started_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            finished_at: None,
            rating: None,
            quick_reviews: Vec::new(),
            created_at: None,
        },
    )
    .await;

    // One completed reading
    let _finished = create_entity::<_, booklog::domain::readings::Reading>(
        &app,
        "/readings",
        &booklog::domain::readings::NewReading {
            user_id: booklog::domain::ids::UserId::new(1),
            book_id: book2.id,
            status: booklog::domain::readings::ReadingStatus::Read,
            format: None,
            started_at: None,
            finished_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
            rating: None,
            quick_reviews: Vec::new(),
            created_at: None,
        },
    )
    .await;

    let client = Client::new();
    let response = client
        .post(app.api_url("/stats/recompute"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let reading = &body["reading"];

    assert_eq!(
        reading["books_all_time"], 1,
        "only completed readings count"
    );
    assert_eq!(reading["books_abandoned"], 1);
}

#[tokio::test]
async fn stats_computes_averages_across_multiple_readings() {
    let app = spawn_app_with_auth().await;

    let author = create_default_author(&app).await;

    // Create two books with page counts
    let book1 = create_entity::<_, booklog::domain::book_items::Book>(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "Book One".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author.id,
                role: booklog::domain::book_items::AuthorRole::Author,
            }],
            isbn: None,
            description: None,
            page_count: Some(100),
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            created_at: None,
        },
    )
    .await;

    let book2 = create_entity::<_, booklog::domain::book_items::Book>(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "Book Two".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author.id,
                role: booklog::domain::book_items::AuthorRole::Author,
            }],
            isbn: None,
            description: None,
            page_count: Some(500),
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            created_at: None,
        },
    )
    .await;

    // Reading 1: rating 3.0, physical, 10 days to finish
    let _r1 = create_entity::<_, booklog::domain::readings::Reading>(
        &app,
        "/readings",
        &booklog::domain::readings::NewReading {
            user_id: booklog::domain::ids::UserId::new(1),
            book_id: book1.id,
            status: booklog::domain::readings::ReadingStatus::Read,
            format: Some(booklog::domain::readings::ReadingFormat::Physical),
            started_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            finished_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 11).unwrap()),
            rating: Some(3.0),
            quick_reviews: Vec::new(),
            created_at: None,
        },
    )
    .await;

    // Reading 2: rating 5.0, ereader, 20 days to finish
    let _r2 = create_entity::<_, booklog::domain::readings::Reading>(
        &app,
        "/readings",
        &booklog::domain::readings::NewReading {
            user_id: booklog::domain::ids::UserId::new(1),
            book_id: book2.id,
            status: booklog::domain::readings::ReadingStatus::Read,
            format: Some(booklog::domain::readings::ReadingFormat::EReader),
            started_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            finished_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 21).unwrap()),
            rating: Some(5.0),
            quick_reviews: Vec::new(),
            created_at: None,
        },
    )
    .await;

    let client = Client::new();
    let response = client
        .post(app.api_url("/stats/recompute"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let reading = &body["reading"];
    let book_summary = &body["book_summary"];

    // Average rating: (3.0 + 5.0) / 2 = 4.0
    let avg = reading["average_rating"].as_f64().unwrap();
    assert!(
        (avg - 4.0).abs() < 0.01,
        "expected avg rating ~4.0, got {avg}"
    );

    // Average days to finish: (10 + 20) / 2 = 15
    let avg_days = reading["average_days_to_finish"].as_f64().unwrap();
    assert!(
        (avg_days - 15.0).abs() < 0.01,
        "expected avg days ~15, got {avg_days}"
    );

    assert_eq!(reading["books_all_time"], 2);
    assert_eq!(reading["pages_all_time"], 600);

    // Two different formats
    let format_counts = reading["format_counts"].as_array().unwrap();
    assert_eq!(format_counts.len(), 2);

    // Rating distribution should have two entries (3.0 and 5.0)
    let rating_dist = reading["rating_distribution"].as_array().unwrap();
    assert_eq!(rating_dist.len(), 2);

    // Extremes
    let longest = book_summary["longest_book"].as_array().unwrap();
    assert_eq!(longest[0], "Book Two");
    assert_eq!(longest[1], 500);

    let shortest = book_summary["shortest_book"].as_array().unwrap();
    assert_eq!(shortest[0], "Book One");
    assert_eq!(shortest[1], 100);
}

#[tokio::test]
async fn stats_year_scope_excludes_other_years() {
    let app = spawn_app_with_auth().await;

    let author = create_default_author(&app).await;

    let book_2025 = create_entity::<_, booklog::domain::book_items::Book>(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "Old Book".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author.id,
                role: booklog::domain::book_items::AuthorRole::Author,
            }],
            isbn: None,
            description: None,
            page_count: Some(300),
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            created_at: None,
        },
    )
    .await;

    let book_2026 = create_entity::<_, booklog::domain::book_items::Book>(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "New Book".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author.id,
                role: booklog::domain::book_items::AuthorRole::Author,
            }],
            isbn: None,
            description: None,
            page_count: Some(200),
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            created_at: None,
        },
    )
    .await;

    // Finished in 2025
    let _r1 = create_entity::<_, booklog::domain::readings::Reading>(
        &app,
        "/readings",
        &booklog::domain::readings::NewReading {
            user_id: booklog::domain::ids::UserId::new(1),
            book_id: book_2025.id,
            status: booklog::domain::readings::ReadingStatus::Read,
            format: Some(booklog::domain::readings::ReadingFormat::Physical),
            started_at: Some(chrono::NaiveDate::from_ymd_opt(2025, 11, 1).unwrap()),
            finished_at: Some(chrono::NaiveDate::from_ymd_opt(2025, 12, 15).unwrap()),
            rating: Some(3.0),
            quick_reviews: Vec::new(),
            created_at: None,
        },
    )
    .await;

    // Finished in 2026
    let _r2 = create_entity::<_, booklog::domain::readings::Reading>(
        &app,
        "/readings",
        &booklog::domain::readings::NewReading {
            user_id: booklog::domain::ids::UserId::new(1),
            book_id: book_2026.id,
            status: booklog::domain::readings::ReadingStatus::Read,
            format: Some(booklog::domain::readings::ReadingFormat::Audiobook),
            started_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            finished_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 10).unwrap()),
            rating: Some(5.0),
            quick_reviews: Vec::new(),
            created_at: None,
        },
    )
    .await;

    let client = Client::new();
    let session_token = create_session(&app).await;

    // Load year-scoped stats page for 2026
    let response = client
        .get(app.page_url("/stats?year=2026"))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();

    // Should show the 2026 book, not the 2025 one
    assert!(body.contains("New Book") || body.contains("1 book"));
    // The page should not count the 2025 reading
    assert!(!body.contains("2 book"));
}

#[tokio::test]
async fn stats_top_authors_ranks_by_reading_count() {
    let app = spawn_app_with_auth().await;

    let author_a = create_author_with_name(&app, "Alice Author").await;
    let author_b = create_author_with_name(&app, "Bob Writer").await;

    // Two books by Alice, one by Bob
    let book_a1 = create_entity::<_, booklog::domain::book_items::Book>(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "Alice Book 1".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author_a.id,
                role: booklog::domain::book_items::AuthorRole::Author,
            }],
            isbn: None,
            description: None,
            page_count: None,
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            created_at: None,
        },
    )
    .await;

    let book_a2 = create_entity::<_, booklog::domain::book_items::Book>(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "Alice Book 2".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author_a.id,
                role: booklog::domain::book_items::AuthorRole::Author,
            }],
            isbn: None,
            description: None,
            page_count: None,
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            created_at: None,
        },
    )
    .await;

    let book_b1 = create_entity::<_, booklog::domain::book_items::Book>(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "Bob Book 1".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author_b.id,
                role: booklog::domain::book_items::AuthorRole::Author,
            }],
            isbn: None,
            description: None,
            page_count: None,
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            created_at: None,
        },
    )
    .await;

    for book_id in [book_a1.id, book_a2.id, book_b1.id] {
        let _r = create_entity::<_, booklog::domain::readings::Reading>(
            &app,
            "/readings",
            &booklog::domain::readings::NewReading {
                user_id: booklog::domain::ids::UserId::new(1),
                book_id,
                status: booklog::domain::readings::ReadingStatus::Read,
                format: None,
                started_at: None,
                finished_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
                rating: None,
                quick_reviews: Vec::new(),
                created_at: None,
            },
        )
        .await;
    }

    let client = Client::new();
    let response = client
        .post(app.api_url("/stats/recompute"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    let top_authors = body["book_summary"]["top_authors"].as_array().unwrap();

    assert!(!top_authors.is_empty());
    // First entry should be Alice (2 readings) before Bob (1 reading)
    assert_eq!(top_authors[0][0], "Alice Author");
    assert_eq!(top_authors[0][1], 2);
}
