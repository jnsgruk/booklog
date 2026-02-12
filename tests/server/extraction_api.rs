use booklog::infrastructure::ai::{ExtractedAuthor, ExtractedBook};
use reqwest::StatusCode;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::{
    create_default_author, create_default_book, create_genre_with_name,
    spawn_app_with_openrouter_mock,
};

fn mock_openrouter_response(json_content: &str) -> ResponseTemplate {
    let body = serde_json::json!({
        "id": "gen-test",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": json_content
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 100,
            "completion_tokens": 50,
            "total_tokens": 150,
            "cost": 0.001
        }
    });
    ResponseTemplate::new(200).set_body_json(body)
}

// --- extract-author ---

#[tokio::test]
async fn extract_author_returns_json() {
    let app = spawn_app_with_openrouter_mock().await;
    let mock_server = app.mock_server.as_ref().unwrap();

    Mock::given(method("POST"))
        .and(path("/api/v1/chat/completions"))
        .respond_with(mock_openrouter_response(r#"{"name": "Ursula K. Le Guin"}"#))
        .mount(mock_server)
        .await;

    let client = reqwest::Client::new();
    let payload = serde_json::json!({ "prompt": "Ursula K. Le Guin" });

    let response = client
        .post(app.api_url("/extract-author"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let result: ExtractedAuthor = response.json().await.expect("Failed to parse response");
    assert_eq!(result.name.as_deref(), Some("Ursula K. Le Guin"));
}

#[tokio::test]
async fn extract_author_returns_datastar_signals() {
    let app = spawn_app_with_openrouter_mock().await;
    let mock_server = app.mock_server.as_ref().unwrap();

    Mock::given(method("POST"))
        .and(path("/api/v1/chat/completions"))
        .respond_with(mock_openrouter_response(r#"{"name": "Neil Gaiman"}"#))
        .mount(mock_server)
        .await;

    let client = reqwest::Client::new();
    let payload = serde_json::json!({ "prompt": "Neil Gaiman" });

    let response = client
        .post(app.api_url("/extract-author"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("datastar-request", "true")
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["_authorName"], "Neil Gaiman");
}

#[tokio::test]
async fn extract_author_requires_auth() {
    let app = spawn_app_with_openrouter_mock().await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({ "prompt": "test" });

    let response = client
        .post(app.api_url("/extract-author"))
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn extract_author_rejects_empty_input() {
    let app = spawn_app_with_openrouter_mock().await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({});

    let response = client
        .post(app.api_url("/extract-author"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 400);
}

// --- extract-book ---

#[tokio::test]
async fn extract_book_scan_returns_json() {
    let app = spawn_app_with_openrouter_mock().await;
    let mock_server = app.mock_server.as_ref().unwrap();

    Mock::given(method("POST"))
        .and(path("/api/v1/chat/completions"))
        .respond_with(mock_openrouter_response(
            r#"{"title": "The Left Hand of Darkness", "author_name": "Ursula K. Le Guin", "isbn": "9780441478125", "page_count": 304, "year_published": 1969, "primary_genre": "Science Fiction"}"#,
        ))
        .mount(mock_server)
        .await;

    let client = reqwest::Client::new();
    let payload = serde_json::json!({ "prompt": "The Left Hand of Darkness" });

    let response = client
        .post(app.api_url("/extract-book"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let result: ExtractedBook = response.json().await.expect("Failed to parse response");
    assert_eq!(result.title.as_deref(), Some("The Left Hand of Darkness"));
    assert_eq!(result.author_name.as_deref(), Some("Ursula K. Le Guin"));
    assert_eq!(result.isbn.as_deref(), Some("9780441478125"));
    assert_eq!(result.page_count, Some(304));
    assert_eq!(result.year_published, Some(1969));
}

#[tokio::test]
async fn extract_book_scan_returns_datastar_signals() {
    let app = spawn_app_with_openrouter_mock().await;
    let mock_server = app.mock_server.as_ref().unwrap();

    Mock::given(method("POST"))
        .and(path("/api/v1/chat/completions"))
        .respond_with(mock_openrouter_response(
            r#"{"title": "1984", "author_name": "George Orwell", "page_count": 328}"#,
        ))
        .mount(mock_server)
        .await;

    let client = reqwest::Client::new();
    let payload = serde_json::json!({ "prompt": "1984 by George Orwell" });

    let response = client
        .post(app.api_url("/extract-book"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("datastar-request", "true")
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["_bookTitle"], "1984");
    assert_eq!(body["_authorName"], "George Orwell");
    assert_eq!(body["_bookPages"], "328");
    assert_eq!(body["_scanExtracted"], true);
}

#[tokio::test]
async fn extract_book_scan_requires_auth() {
    let app = spawn_app_with_openrouter_mock().await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({ "prompt": "test" });

    let response = client
        .post(app.api_url("/extract-book"))
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn extract_book_scan_rejects_empty_input() {
    let app = spawn_app_with_openrouter_mock().await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({});

    let response = client
        .post(app.api_url("/extract-book"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn extract_book_scan_matches_existing_author() {
    let app = spawn_app_with_openrouter_mock().await;
    let mock_server = app.mock_server.as_ref().unwrap();

    // Create an existing author
    let author = create_default_author(&app).await;

    Mock::given(method("POST"))
        .and(path("/api/v1/chat/completions"))
        .respond_with(mock_openrouter_response(
            // The extracted author name matches the existing author's name
            r#"{"title": "Some Book", "author_name": "Test Author"}"#,
        ))
        .mount(mock_server)
        .await;

    let client = reqwest::Client::new();
    let payload = serde_json::json!({ "prompt": "Some Book by Test Author" });

    let response = client
        .post(app.api_url("/extract-book"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("datastar-request", "true")
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["_matchedAuthorId"], author.id.into_inner().to_string());
}

#[tokio::test]
async fn extract_book_scan_matches_existing_book() {
    let app = spawn_app_with_openrouter_mock().await;
    let mock_server = app.mock_server.as_ref().unwrap();

    // Create an existing author and book
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;

    Mock::given(method("POST"))
        .and(path("/api/v1/chat/completions"))
        .respond_with(mock_openrouter_response(
            // Both title and author match existing entities
            r#"{"title": "Test Book", "author_name": "Test Author"}"#,
        ))
        .mount(mock_server)
        .await;

    let client = reqwest::Client::new();
    let payload = serde_json::json!({ "prompt": "Test Book by Test Author" });

    let response = client
        .post(app.api_url("/extract-book"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("datastar-request", "true")
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["_matchedAuthorId"], author.id.into_inner().to_string());
    assert_eq!(body["_matchedBookId"], book.id.into_inner().to_string());
}

// --- submit-scan ---

#[tokio::test]
async fn submit_scan_creates_book_and_author() {
    let app = spawn_app_with_openrouter_mock().await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "author_name": "Ursula K. Le Guin",
        "book_title": "The Left Hand of Darkness",
        "book_pages": "304",
        "book_year": "1969",
    });

    let response = client
        .post(app.api_url("/scan"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::CREATED);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(
        body["book_id"].as_i64().is_some(),
        "Expected book_id in response"
    );
    assert!(
        body["redirect"]
            .as_str()
            .is_some_and(|r| r.starts_with("/books/")),
        "Expected redirect URL"
    );

    // Verify the book was actually created
    let books_response = client
        .get(app.api_url("/books"))
        .send()
        .await
        .expect("Failed to list books");
    let books: Vec<serde_json::Value> = books_response.json().await.unwrap();
    assert!(
        books
            .iter()
            .any(|b| b["title"] == "The Left Hand of Darkness"),
        "Book should exist after scan submission"
    );
}

#[tokio::test]
async fn submit_scan_rejects_empty_title() {
    let app = spawn_app_with_openrouter_mock().await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "author_name": "Some Author",
        "book_title": "",
    });

    let response = client
        .post(app.api_url("/scan"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn submit_scan_rejects_empty_author() {
    let app = spawn_app_with_openrouter_mock().await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "author_name": "",
        "book_title": "Some Book",
    });

    let response = client
        .post(app.api_url("/scan"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn submit_scan_with_matched_book_id_uses_existing_book() {
    let app = spawn_app_with_openrouter_mock().await;
    let client = reqwest::Client::new();

    // Create an author and book first
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;

    let payload = serde_json::json!({
        "author_name": "Test Author",
        "book_title": "Test Book",
        "matched_book_id": book.id.into_inner().to_string(),
    });

    let response = client
        .post(app.api_url("/scan"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::CREATED);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(
        body["book_id"].as_i64(),
        Some(book.id.into_inner()),
        "Should use existing book ID"
    );
}

#[tokio::test]
async fn submit_scan_reuses_existing_author_by_name() {
    let app = spawn_app_with_openrouter_mock().await;
    let client = reqwest::Client::new();

    // Create an author first
    let _author = create_default_author(&app).await;

    let payload = serde_json::json!({
        "author_name": "Test Author",
        "book_title": "A Different Book",
    });

    let response = client
        .post(app.api_url("/scan"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::CREATED);

    // Verify only one author exists (reused, not duplicated)
    let authors_response = client
        .get(app.api_url("/authors"))
        .send()
        .await
        .expect("Failed to list authors");
    let authors: Vec<serde_json::Value> = authors_response.json().await.unwrap();
    let test_authors: Vec<_> = authors
        .iter()
        .filter(|a| a["name"] == "Test Author")
        .collect();
    assert_eq!(
        test_authors.len(),
        1,
        "Should reuse existing author, not create a duplicate"
    );
}

#[tokio::test]
async fn submit_scan_with_extraction_creates_book() {
    let app = spawn_app_with_openrouter_mock().await;
    let mock_server = app.mock_server.as_ref().unwrap();

    Mock::given(method("POST"))
        .and(path("/api/v1/chat/completions"))
        .respond_with(mock_openrouter_response(
            r#"{"title": "Dune", "author_name": "Frank Herbert", "page_count": 412, "year_published": 1965}"#,
        ))
        .mount(mock_server)
        .await;

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "prompt": "Dune by Frank Herbert",
    });

    let response = client
        .post(app.api_url("/scan"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::CREATED);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(body["book_id"].as_i64().is_some());

    // Verify the extracted data was used
    let books_response = client
        .get(app.api_url("/books"))
        .send()
        .await
        .expect("Failed to list books");
    let books: Vec<serde_json::Value> = books_response.json().await.unwrap();
    assert!(
        books.iter().any(|b| b["title"] == "Dune"),
        "Book should be created from AI extraction"
    );
}

// --- scan with genre auto-creation ---

#[tokio::test]
async fn submit_scan_with_genre_name_auto_creates_genre() {
    let app = spawn_app_with_openrouter_mock().await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "author_name": "Test Author",
        "book_title": "Genre Auto-Create Test",
        "book_primary_genre_name": "Cyberpunk",
    });

    let response = client
        .post(app.api_url("/scan"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::CREATED);

    // Verify the genre was auto-created
    let genres_response = client
        .get(app.api_url("/genres"))
        .send()
        .await
        .expect("Failed to list genres");
    let genres: Vec<serde_json::Value> = genres_response.json().await.unwrap();
    assert!(
        genres.iter().any(|g| g["name"] == "Cyberpunk"),
        "Genre should be auto-created from scan submission"
    );
}

#[tokio::test]
async fn submit_scan_with_existing_genre_name_reuses_genre() {
    let app = spawn_app_with_openrouter_mock().await;
    let client = reqwest::Client::new();

    // Create genre first
    let _genre = create_genre_with_name(&app, "Fantasy").await;

    let payload = serde_json::json!({
        "author_name": "Test Author",
        "book_title": "Genre Reuse Test",
        "book_primary_genre_name": "Fantasy",
    });

    let response = client
        .post(app.api_url("/scan"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::CREATED);

    // Verify no duplicate genre was created
    let genres_response = client
        .get(app.api_url("/genres"))
        .send()
        .await
        .expect("Failed to list genres");
    let genres: Vec<serde_json::Value> = genres_response.json().await.unwrap();
    let fantasy_genres: Vec<_> = genres.iter().filter(|g| g["name"] == "Fantasy").collect();
    assert_eq!(
        fantasy_genres.len(),
        1,
        "Should reuse existing genre, not create a duplicate"
    );
}

#[tokio::test]
async fn extract_book_scan_returns_genre_in_signals() {
    let app = spawn_app_with_openrouter_mock().await;
    let mock_server = app.mock_server.as_ref().unwrap();

    Mock::given(method("POST"))
        .and(path("/api/v1/chat/completions"))
        .respond_with(mock_openrouter_response(
            r#"{"title": "Neuromancer", "author_name": "William Gibson", "primary_genre": "Cyberpunk", "secondary_genre": "Noir"}"#,
        ))
        .mount(mock_server)
        .await;

    let client = reqwest::Client::new();
    let payload = serde_json::json!({ "prompt": "Neuromancer by William Gibson" });

    let response = client
        .post(app.api_url("/extract-book"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("datastar-request", "true")
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["_bookPrimaryGenre"], "Cyberpunk");
    assert_eq!(body["_bookSecondaryGenre"], "Noir");
}

#[tokio::test]
async fn extract_book_scan_resolves_existing_genre_id_in_signals() {
    let app = spawn_app_with_openrouter_mock().await;
    let mock_server = app.mock_server.as_ref().unwrap();

    // Create a genre first
    let genre = create_genre_with_name(&app, "Science Fiction").await;

    Mock::given(method("POST"))
        .and(path("/api/v1/chat/completions"))
        .respond_with(mock_openrouter_response(
            r#"{"title": "Dune", "author_name": "Frank Herbert", "primary_genre": "Science Fiction"}"#,
        ))
        .mount(mock_server)
        .await;

    let client = reqwest::Client::new();
    let payload = serde_json::json!({ "prompt": "Dune" });

    let response = client
        .post(app.api_url("/extract-book"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("datastar-request", "true")
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["_bookPrimaryGenre"], "Science Fiction");
    assert_eq!(
        body["_bookPrimaryGenreId"],
        genre.id.into_inner().to_string()
    );
}

// --- cover suggestion endpoints ---

#[tokio::test]
async fn get_cover_suggestion_returns_404_for_nonexistent_id() {
    let app = spawn_app_with_openrouter_mock().await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.api_url("/cover-suggestions/nonexistent"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_cover_suggestion_thumbnail_returns_404_for_nonexistent_id() {
    let app = spawn_app_with_openrouter_mock().await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.api_url("/cover-suggestions/nonexistent/thumbnail"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
