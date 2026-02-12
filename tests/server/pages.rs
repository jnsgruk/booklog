use reqwest::redirect::Policy;

use crate::helpers::{
    assert_full_page, create_default_author, create_default_book, create_default_genre,
    create_default_reading, create_session, spawn_app, spawn_app_with_auth,
};
use reqwest::StatusCode;

#[tokio::test]
async fn homepage_returns_200_with_empty_database() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url("/"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
}

#[tokio::test]
async fn homepage_returns_200_with_data() {
    let app = spawn_app_with_auth().await;

    let author = create_default_author(&app).await;
    let _book = create_default_book(&app, author.id).await;

    let client = reqwest::Client::new();
    let response = client
        .get(app.page_url("/"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
    assert!(
        body.contains("Test Author"),
        "Homepage should contain author name"
    );
}

#[tokio::test]
async fn homepage_shows_stats_counts() {
    let app = spawn_app_with_auth().await;
    let session_token = create_session(&app).await;

    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    // Creating a reading also creates a library entry and a "currently reading" item
    let _reading = create_default_reading(&app, book.id).await;

    let client = reqwest::Client::new();
    let response = client
        .get(app.page_url("/"))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    // Stats section should show per-user counts (library, wishlist, currently reading)
    assert!(body.contains("Library"), "Should show Library stat card");
    assert!(body.contains("Reading"), "Should show Reading stat card");
}

#[tokio::test]
async fn login_page_returns_200() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url("/login"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
}

#[tokio::test]
async fn add_page_redirects_unauthenticated_to_login() {
    let app = spawn_app().await;
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .build()
        .expect("Failed to build client");

    let response = client
        .get(app.page_url("/add"))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(
        response.status().is_redirection(),
        "Expected redirect, got {}",
        response.status()
    );
    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok());
    assert_eq!(location, Some("/login"));
}

#[tokio::test]
async fn add_page_returns_200_when_authenticated() {
    let app = spawn_app_with_auth().await;
    let session_token = create_session(&app).await;

    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .build()
        .expect("Failed to build client");

    let response = client
        .get(app.page_url("/add"))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
}

#[tokio::test]
async fn admin_page_redirects_unauthenticated_to_login() {
    let app = spawn_app().await;
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .build()
        .expect("Failed to build client");

    let response = client
        .get(app.page_url("/admin"))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(
        response.status().is_redirection(),
        "Expected redirect, got {}",
        response.status()
    );
    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok());
    assert_eq!(location, Some("/login"));
}

#[tokio::test]
async fn admin_page_returns_200_when_authenticated() {
    let app = spawn_app_with_auth().await;
    let session_token = create_session(&app).await;

    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .build()
        .expect("Failed to build client");

    let response = client
        .get(app.page_url("/admin"))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
}

#[tokio::test]
async fn logout_redirects_to_homepage() {
    let app = spawn_app_with_auth().await;
    let session_token = create_session(&app).await;

    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .build()
        .expect("Failed to build client");

    let response = client
        .post(app.page_url("/logout"))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(
        response.status().is_redirection(),
        "Expected redirect, got {}",
        response.status()
    );
    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok());
    assert_eq!(location, Some("/"));
}

#[tokio::test]
async fn book_detail_shows_on_shelf_for_shelved_book() {
    let app = spawn_app_with_auth().await;
    let session_token = create_session(&app).await;

    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;

    // Shelve the book (on shelf, no reading)
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
        .expect("Failed to shelve book");
    assert_eq!(response.status(), 201);

    // Fetch the book detail page as authenticated user
    let response = client
        .get(app.page_url(&format!("/books/{}", book.id)))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to fetch book detail");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
    assert!(
        body.contains("On Shelf"),
        "Expected 'On Shelf' status on book detail page, got: {body}"
    );
    assert!(
        body.contains("Start Reading"),
        "Expected 'Start Reading' action for on-shelf book, got: {body}"
    );
}

#[tokio::test]
async fn book_detail_shows_finish_reading_for_active_reading() {
    let app = spawn_app_with_auth().await;
    let session_token = create_session(&app).await;

    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let reading = create_default_reading(&app, book.id).await;

    let client = reqwest::Client::new();
    let response = client
        .get(app.page_url(&format!("/books/{}", book.id)))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to fetch book detail");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
    assert!(
        body.contains("Finish Reading"),
        "Expected 'Finish Reading' action for actively reading book, got: {body}"
    );
    assert!(
        body.contains(&format!("/readings/{}/edit?finish=true", reading.id)),
        "Expected finish link to point to reading edit with finish=true, got: {body}"
    );
}

#[tokio::test]
async fn book_detail_shows_wishlist_for_wishlisted_book() {
    let app = spawn_app_with_auth().await;
    let session_token = create_session(&app).await;

    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;

    // Add to wishlist via user-books API
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "book_id": book.id,
        "shelf": "wishlist"
    });
    let response = client
        .post(app.api_url("/user-books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to create wishlist entry");
    assert_eq!(response.status(), 201);

    // Fetch the book detail page as authenticated user
    let response = client
        .get(app.page_url(&format!("/books/{}", book.id)))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to fetch book detail");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
    assert!(
        body.contains("Wishlist"),
        "Expected 'Wishlist' heading on book detail page for wishlisted book, got: {body}"
    );
    assert!(
        body.contains("Start Reading"),
        "Expected 'Start Reading' action for wishlisted book, got: {body}"
    );
    assert!(
        !body.contains("On Shelf"),
        "Should not show 'On Shelf' for a wishlisted book, got: {body}"
    );
}

// --- Detail page tests ---

#[tokio::test]
async fn author_detail_page_returns_200() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url(&format!("/authors/{}", author.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
    assert!(
        body.contains("Test Author"),
        "Detail page should contain author name"
    );
}

#[tokio::test]
async fn genre_detail_page_returns_200() {
    let app = spawn_app_with_auth().await;
    let genre = create_default_genre(&app).await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url(&format!("/genres/{}", genre.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
    assert!(
        body.contains("Test Genre"),
        "Detail page should contain genre name"
    );
}

#[tokio::test]
async fn reading_detail_page_returns_200() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let reading = create_default_reading(&app, book.id).await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url(&format!("/readings/{}", reading.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
    assert!(
        body.contains("Test Book"),
        "Reading detail page should contain book title"
    );
}

#[tokio::test]
async fn nonexistent_author_returns_404() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url("/authors/99999"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn nonexistent_book_returns_404() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url("/books/99999"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn nonexistent_genre_returns_404() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url("/genres/99999"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn nonexistent_reading_returns_404() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url("/readings/99999"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 404);
}

// --- Edit page tests ---

#[tokio::test]
async fn author_edit_page_returns_200_when_authenticated() {
    let app = spawn_app_with_auth().await;
    let session_token = create_session(&app).await;
    let author = create_default_author(&app).await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url(&format!("/authors/{}/edit", author.id)))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
    assert!(
        body.contains("Test Author"),
        "Edit page should contain author name"
    );
}

#[tokio::test]
async fn author_edit_page_redirects_unauthenticated() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let response = client
        .get(app.page_url(&format!("/authors/{}/edit", author.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_redirection());
    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok());
    assert_eq!(location, Some("/login"));
}

#[tokio::test]
async fn book_edit_page_returns_200_when_authenticated() {
    let app = spawn_app_with_auth().await;
    let session_token = create_session(&app).await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url(&format!("/books/{}/edit", book.id)))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
    assert!(
        body.contains("Test Book"),
        "Edit page should contain book title"
    );
}

#[tokio::test]
async fn book_edit_page_redirects_unauthenticated() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let response = client
        .get(app.page_url(&format!("/books/{}/edit", book.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_redirection());
    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok());
    assert_eq!(location, Some("/login"));
}

#[tokio::test]
async fn reading_edit_page_returns_200_when_authenticated() {
    let app = spawn_app_with_auth().await;
    let session_token = create_session(&app).await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let reading = create_default_reading(&app, book.id).await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url(&format!("/readings/{}/edit", reading.id)))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
}

#[tokio::test]
async fn reading_edit_page_redirects_unauthenticated() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let reading = create_default_reading(&app, book.id).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let response = client
        .get(app.page_url(&format!("/readings/{}/edit", reading.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_redirection());
    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok());
    assert_eq!(location, Some("/login"));
}

#[tokio::test]
async fn genre_edit_page_returns_200_when_authenticated() {
    let app = spawn_app_with_auth().await;
    let session_token = create_session(&app).await;
    let genre = create_default_genre(&app).await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url(&format!("/genres/{}/edit", genre.id)))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
    assert!(
        body.contains("Test Genre"),
        "Edit page should contain genre name"
    );
}

#[tokio::test]
async fn genre_edit_page_redirects_unauthenticated() {
    let app = spawn_app_with_auth().await;
    let genre = create_default_genre(&app).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let response = client
        .get(app.page_url(&format!("/genres/{}/edit", genre.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_redirection());
    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok());
    assert_eq!(location, Some("/login"));
}

// --- Start book page tests ---

#[tokio::test]
async fn start_book_page_returns_200_when_authenticated() {
    let app = spawn_app_with_auth().await;
    let session_token = create_session(&app).await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url(&format!("/books/{}/start", book.id)))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
    assert!(
        body.contains("Test Book"),
        "Start book page should contain book title"
    );
}

#[tokio::test]
async fn start_book_page_redirects_unauthenticated() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let response = client
        .get(app.page_url(&format!("/books/{}/start", book.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_redirection());
    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok());
    assert_eq!(location, Some("/login"));
}

#[tokio::test]
async fn start_book_page_returns_404_for_nonexistent_book() {
    let app = spawn_app_with_auth().await;
    let session_token = create_session(&app).await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url("/books/99999/start"))
        .header("Cookie", format!("booklog_session={session_token}"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 404);
}

// --- Register and CLI callback page tests ---

#[tokio::test]
async fn register_page_with_invalid_token_returns_404() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url("/register/invalid-token"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn cli_callback_page_returns_200() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.page_url("/auth/cli-callback"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.text().await.expect("Failed to read body");
    assert_full_page(&body);
}
