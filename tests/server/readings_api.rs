use crate::helpers::{
    create_default_author, create_default_book, create_default_reading, create_non_admin_token,
    spawn_app, spawn_app_with_auth,
};
use booklog::domain::ids::UserId;
use booklog::domain::readings::{
    NewReading, Reading, ReadingFormat, ReadingStatus, ReadingWithBook, UpdateReading,
};
use booklog::domain::user_books::UserBook;

#[tokio::test]
async fn creating_a_reading_returns_a_201_for_valid_data() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    let new_reading = NewReading {
        user_id: UserId::new(1),
        book_id: book.id,
        status: ReadingStatus::Reading,
        format: Some(ReadingFormat::Physical),
        started_at: Some(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        finished_at: None,
        rating: None,
        quick_reviews: Vec::new(),
        created_at: None,
    };

    let response = client
        .post(app.api_url("/readings"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_reading)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 201);

    let reading: Reading = response.json().await.expect("Failed to parse response");
    assert_eq!(reading.book_id, book.id);
    assert_eq!(reading.status, ReadingStatus::Reading);
    assert_eq!(reading.format, Some(ReadingFormat::Physical));
}

#[tokio::test]
async fn creating_a_reading_without_auth_returns_401() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let new_reading = serde_json::json!({
        "book_id": 123,
        "status": "reading",
        "format": "physical"
    });

    let response = client
        .post(app.api_url("/readings"))
        .json(&new_reading)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn listing_readings_returns_200_and_correct_data() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    let new_reading = NewReading {
        user_id: UserId::new(1),
        book_id: book.id,
        status: ReadingStatus::Reading,
        format: Some(ReadingFormat::EReader),
        started_at: None,
        finished_at: None,
        rating: None,
        quick_reviews: Vec::new(),
        created_at: None,
    };

    client
        .post(app.api_url("/readings"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_reading)
        .send()
        .await
        .expect("Failed to create reading");

    let response = client
        .get(app.api_url("/readings"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let readings: Vec<ReadingWithBook> = response.json().await.expect("Failed to parse response");
    assert_eq!(readings.len(), 1);
    assert_eq!(readings[0].book_title, book.title);
}

#[tokio::test]
async fn getting_a_reading_returns_200_for_valid_id() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let reading = create_default_reading(&app, book.id).await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.api_url(&format!("/readings/{}", reading.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn updating_a_reading_returns_200_and_updates_data() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let reading = create_default_reading(&app, book.id).await;
    let client = reqwest::Client::new();

    let update_payload = UpdateReading {
        status: Some(ReadingStatus::Read),
        format: Some(ReadingFormat::EReader),
        rating: Some(5.0),
        finished_at: Some(chrono::NaiveDate::from_ymd_opt(2025, 2, 1).unwrap()),
        ..Default::default()
    };

    let response = client
        .put(app.api_url(&format!("/readings/{}", reading.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&update_payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);
    let updated_reading: Reading = response.json().await.expect("Failed to parse response");
    assert_eq!(updated_reading.status, ReadingStatus::Read);
    assert_eq!(updated_reading.rating, Some(5.0));
    assert_eq!(updated_reading.format, Some(ReadingFormat::EReader));
}

#[tokio::test]
async fn deleting_a_reading_returns_204() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let reading = create_default_reading(&app, book.id).await;
    let client = reqwest::Client::new();

    let response = client
        .delete(app.api_url(&format!("/readings/{}", reading.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 204);

    let get_response = client
        .get(app.api_url(&format!("/readings/{}", reading.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(get_response.status(), 404);
}

// --- Rating validation tests ---

#[tokio::test]
async fn creating_a_reading_with_invalid_rating_returns_400() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    // Rating too high
    let payload = serde_json::json!({
        "book_id": book.id,
        "status": "reading",
        "format": "physical",
        "rating": 6
    });

    let response = client
        .post(app.api_url("/readings"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn creating_a_reading_with_zero_rating_treats_as_no_rating() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "book_id": book.id,
        "status": "reading",
        "format": "physical",
        "rating": 0
    });

    let response = client
        .post(app.api_url("/readings"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 201);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["reading"]["rating"].is_null());
}

#[tokio::test]
async fn creating_a_reading_with_negative_rating_returns_400() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "book_id": book.id,
        "status": "reading",
        "format": "physical",
        "rating": -1
    });

    let response = client
        .post(app.api_url("/readings"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn creating_a_reading_with_valid_rating_succeeds() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    for valid_rating in [0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0] {
        let new_reading = NewReading {
            user_id: UserId::new(1),
            book_id: book.id,
            status: ReadingStatus::Reading,
            format: Some(ReadingFormat::Physical),
            started_at: None,
            finished_at: None,
            rating: Some(valid_rating),
            quick_reviews: Vec::new(),
            created_at: None,
        };

        let response = client
            .post(app.api_url("/readings"))
            .bearer_auth(app.auth_token.as_ref().unwrap())
            .json(&new_reading)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            response.status(),
            201,
            "Rating {valid_rating} should be valid"
        );
    }
}

#[tokio::test]
async fn updating_a_reading_with_invalid_rating_returns_400() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let reading = create_default_reading(&app, book.id).await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({ "rating": 10 });

    let response = client
        .put(app.api_url(&format!("/readings/{}", reading.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn creating_a_reading_with_non_half_star_rating_returns_400() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    for invalid_rating in [1.3, 2.7, 0.1] {
        let payload = serde_json::json!({
            "book_id": book.id,
            "status": "reading",
            "rating": invalid_rating
        });

        let response = client
            .post(app.api_url("/readings"))
            .bearer_auth(app.auth_token.as_ref().unwrap())
            .json(&payload)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            response.status(),
            400,
            "Rating {invalid_rating} should be invalid"
        );
    }
}

// --- Format validation tests ---

#[tokio::test]
async fn creating_a_reading_without_format_succeeds() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "book_id": book.id,
        "status": "reading"
    });

    let response = client
        .post(app.api_url("/readings"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 201);
}

// --- Listing filter tests ---

#[tokio::test]
async fn listing_readings_filtered_by_book_id_returns_only_matching() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let client = reqwest::Client::new();

    // Create two books
    let book1 = crate::helpers::create_default_book(&app, author.id).await;

    let book2: booklog::domain::book_items::Book = crate::helpers::create_entity(
        &app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "Second Book".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id: author.id,
                role: booklog::domain::book_items::AuthorRole::default(),
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

    // Create a reading for each book
    create_default_reading(&app, book1.id).await;
    create_default_reading(&app, book2.id).await;

    // Filter by book1
    let response = client
        .get(app.api_url(&format!("/readings?book_id={}", book1.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);
    let readings: Vec<ReadingWithBook> = response.json().await.expect("Failed to parse response");
    assert_eq!(readings.len(), 1);
    assert_eq!(readings[0].reading.book_id, book1.id);
}

// --- On shelf tests ---

#[tokio::test]
async fn on_shelf_creates_user_book_without_reading() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "book_id": book.id,
        "status": "on_shelf"
    });

    let response = client
        .post(app.api_url("/readings"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 201);

    let user_book: UserBook = response.json().await.expect("Failed to parse user_book");
    assert_eq!(user_book.book_id, book.id);
    assert_eq!(user_book.shelf.as_str(), "library");

    // Verify no reading was created
    let readings_response = client
        .get(app.api_url(&format!("/readings?book_id={}", book.id)))
        .send()
        .await
        .expect("Failed to list readings");

    let readings: Vec<ReadingWithBook> = readings_response
        .json()
        .await
        .expect("Failed to parse readings");
    assert!(
        readings.is_empty(),
        "Expected no readings for on-shelf book"
    );
}

#[tokio::test]
async fn on_shelf_duplicate_redirects_to_book() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let payload = serde_json::json!({
        "book_id": book.id,
        "status": "on_shelf"
    });

    // First shelf
    let response = client
        .post(app.api_url("/readings"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");
    assert_eq!(response.status(), 201);

    // Second shelf — should redirect
    let response = client
        .post(app.api_url("/readings"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert!(
        response.status().is_redirection(),
        "Expected redirect for duplicate on-shelf, got {}",
        response.status()
    );
}

#[tokio::test]
async fn on_shelf_with_book_club_flag() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "book_id": book.id,
        "status": "on_shelf",
        "book_club": true
    });

    let response = client
        .post(app.api_url("/readings"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 201);

    let user_book: UserBook = response.json().await.expect("Failed to parse user_book");
    assert!(user_book.book_club, "Expected book_club to be true");
}

// --- Ownership (IDOR) tests ---

#[tokio::test]
async fn updating_another_users_reading_returns_404() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let reading = create_default_reading(&app, book.id).await;
    let client = reqwest::Client::new();

    let other_user_token = create_non_admin_token(&app).await;

    let update_payload = UpdateReading {
        rating: Some(1.0),
        ..Default::default()
    };

    let response = client
        .put(app.api_url(&format!("/readings/{}", reading.id)))
        .bearer_auth(&other_user_token)
        .json(&update_payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn deleting_another_users_reading_returns_404() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let reading = create_default_reading(&app, book.id).await;
    let client = reqwest::Client::new();

    let other_user_token = create_non_admin_token(&app).await;

    let response = client
        .delete(app.api_url(&format!("/readings/{}", reading.id)))
        .bearer_auth(&other_user_token)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 404);

    // Verify the reading still exists
    let get_response = client
        .get(app.api_url(&format!("/readings/{}", reading.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(get_response.status(), 200);
}
