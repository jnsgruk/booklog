//! Integration tests for Datastar partial rendering.
//!
//! These tests verify that endpoints return HTML fragments with correct
//! Datastar headers when the `datastar-request: true` header is present.

use crate::helpers::{
    TestApp, assert_datastar_headers, assert_datastar_headers_with_mode, assert_full_page,
    assert_html_fragment, create_default_author, create_default_book, create_default_genre,
    create_default_reading, spawn_app_with_auth,
};
use booklog::domain::authors::NewAuthor;
use reqwest::Client;

use crate::test_macros::define_datastar_entity_tests;

// ============================================================================
// Setup functions (return entity ID as String for delete tests)
// ============================================================================

async fn create_author_entity(app: &TestApp) -> String {
    create_default_author(app).await.id.to_string()
}

async fn create_book_entity(app: &TestApp) -> String {
    let author = create_default_author(app).await;
    create_default_book(app, author.id).await.id.to_string()
}

async fn create_reading_entity(app: &TestApp) -> String {
    let author = create_default_author(app).await;
    let book = create_default_book(app, author.id).await;
    create_default_reading(app, book.id).await.id.to_string()
}

async fn create_genre_entity(app: &TestApp) -> String {
    create_default_genre(app).await.id.to_string()
}

// ============================================================================
// Genre Datastar tests (macro-generated)
// ============================================================================

define_datastar_entity_tests!(
    entity: genre,
    type_param: "genres",
    api_path: "/genres",
    list_element: r#"id="genre-list""#,
    selector: "#genre-list",
    setup: create_genre_entity
);

// ============================================================================
// Data page (library) with/without datastar header
// ============================================================================

#[tokio::test]
async fn library_list_with_datastar_header_returns_fragment() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .get(format!("{}/data", app.address))
        .header("datastar-request", "true")
        .send()
        .await
        .expect("failed to fetch library");

    assert_eq!(response.status(), 200);
    assert_datastar_headers_with_mode(&response, "#data-content", "inner");

    let body = response.text().await.expect("failed to read body");
    assert_html_fragment(&body);
    assert!(
        body.contains(r#"id="user-book-list""#),
        "Fragment should contain the user-book-list element"
    );
}

#[tokio::test]
async fn library_list_without_datastar_header_returns_full_page() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .get(format!("{}/data", app.address))
        .send()
        .await
        .expect("failed to fetch library");

    assert_eq!(response.status(), 200);
    assert!(response.headers().get("datastar-selector").is_none());

    let body = response.text().await.expect("failed to read body");
    assert_full_page(&body);
}

// ============================================================================
// Delete with datastar header returns fragment
// ============================================================================

#[tokio::test]
async fn authors_delete_with_datastar_header_returns_fragment() {
    let app = spawn_app_with_auth().await;
    let entity_id = create_author_entity(&app).await;
    let client = Client::new();

    let response = client
        .delete(app.api_url(&format!("/authors/{}", entity_id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("datastar-request", "true")
        .header("referer", format!("{}/data?type=authors", app.address))
        .send()
        .await
        .expect("failed to delete author");

    assert_eq!(response.status(), 200);
    assert_datastar_headers(&response, "#author-list");

    let body = response.text().await.expect("failed to read body");
    assert_html_fragment(&body);
}

#[tokio::test]
async fn books_delete_with_datastar_header_returns_fragment() {
    let app = spawn_app_with_auth().await;
    let entity_id = create_book_entity(&app).await;
    let client = Client::new();

    let response = client
        .delete(app.api_url(&format!("/books/{}", entity_id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("datastar-request", "true")
        .header("referer", format!("{}/data?type=books", app.address))
        .send()
        .await
        .expect("failed to delete book");

    assert_eq!(response.status(), 200);
    assert_datastar_headers(&response, "#book-list");

    let body = response.text().await.expect("failed to read body");
    assert_html_fragment(&body);
}

#[tokio::test]
async fn readings_delete_with_datastar_header_returns_fragment() {
    let app = spawn_app_with_auth().await;
    let entity_id = create_reading_entity(&app).await;
    let client = Client::new();

    let response = client
        .delete(app.api_url(&format!("/readings/{}", entity_id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("datastar-request", "true")
        .header("referer", format!("{}/data?type=readings", app.address))
        .send()
        .await
        .expect("failed to delete reading");

    assert_eq!(response.status(), 200);
    assert_datastar_headers(&response, "#reading-list");

    let body = response.text().await.expect("failed to read body");
    assert_html_fragment(&body);
}

// ============================================================================
// Authors (hand-written: create with/without, delete without)
// ============================================================================

#[tokio::test]
async fn authors_create_with_datastar_header_returns_fragment() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let new_author = NewAuthor {
        name: "Datastar Test Author".to_string(),
        created_at: None,
    };

    let response = client
        .post(app.api_url("/authors"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("datastar-request", "true")
        .header("referer", format!("{}/data?type=authors", app.address))
        .json(&new_author)
        .send()
        .await
        .expect("failed to create author");

    assert_eq!(response.status(), 200);
    assert_datastar_headers(&response, "#author-list");

    let body = response.text().await.expect("failed to read body");
    assert_html_fragment(&body);
    assert!(
        body.contains("Datastar Test Author"),
        "Fragment should include created author"
    );
}

#[tokio::test]
async fn authors_create_without_datastar_header_returns_json() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let new_author = NewAuthor {
        name: "JSON Test Author".to_string(),
        created_at: None,
    };

    let response = client
        .post(app.api_url("/authors"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_author)
        .send()
        .await
        .expect("failed to create author");

    assert_eq!(response.status(), 201);
    assert!(response.headers().get("datastar-selector").is_none());

    let author: booklog::domain::authors::Author =
        response.json().await.expect("failed to parse JSON");
    assert_eq!(author.name, "JSON Test Author");
}

#[tokio::test]
async fn authors_delete_without_datastar_header_returns_204() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let client = Client::new();

    let response = client
        .delete(app.api_url(&format!("/authors/{}", author.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("failed to delete author");

    assert_eq!(response.status(), 204);
    assert!(response.headers().get("datastar-selector").is_none());
}

// ============================================================================
// Authors (update with/without datastar)
// ============================================================================

#[tokio::test]
async fn authors_update_with_datastar_header_returns_redirect_script() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let client = Client::new();

    let update = serde_json::json!({
        "name": "Updated Author",
    });

    let response = client
        .put(app.api_url(&format!("/authors/{}", author.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("datastar-request", "true")
        .json(&update)
        .send()
        .await
        .expect("failed to update author");

    assert_eq!(response.status(), 200);
    assert_datastar_headers_with_mode(&response, "body", "append");

    let body = response.text().await.expect("failed to read body");
    assert!(
        body.contains("window.location"),
        "Expected redirect script in body"
    );
}

#[tokio::test]
async fn authors_update_without_datastar_header_returns_json() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let client = Client::new();

    let update = serde_json::json!({
        "name": "JSON Updated Author",
    });

    let response = client
        .put(app.api_url(&format!("/authors/{}", author.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&update)
        .send()
        .await
        .expect("failed to update author");

    assert_eq!(response.status(), 200);
    assert!(response.headers().get("datastar-selector").is_none());

    let updated: booklog::domain::authors::Author =
        response.json().await.expect("failed to parse JSON");
    assert_eq!(updated.name, "JSON Updated Author");
}

// ============================================================================
// Readings (hand-written: update with datastar)
// ============================================================================

#[tokio::test]
async fn readings_update_with_datastar_header_returns_redirect_script() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let reading = create_default_reading(&app, book.id).await;
    let client = Client::new();

    let update = serde_json::json!({
        "rating": 5.0,
    });

    let response = client
        .put(app.api_url(&format!("/readings/{}", reading.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("datastar-request", "true")
        .json(&update)
        .send()
        .await
        .expect("failed to update reading");

    assert_eq!(response.status(), 200);
    assert_datastar_headers_with_mode(&response, "body", "append");

    let body = response.text().await.expect("failed to read body");
    assert!(
        body.contains("window.location"),
        "Expected redirect script in body"
    );
}

// ============================================================================
// Timeline
// ============================================================================

#[tokio::test]
async fn timeline_with_datastar_header_returns_fragment() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .get(format!("{}/timeline", app.address))
        .header("datastar-request", "true")
        .send()
        .await
        .expect("failed to fetch timeline");

    assert_eq!(response.status(), 200);
    assert_datastar_headers(&response, "#timeline-loader");

    let body = response.text().await.expect("failed to read body");
    assert_html_fragment(&body);
}

#[tokio::test]
async fn timeline_without_datastar_header_returns_full_page() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .get(format!("{}/timeline", app.address))
        .send()
        .await
        .expect("failed to fetch timeline");

    assert_eq!(response.status(), 200);
    assert!(response.headers().get("datastar-selector").is_none());

    let body = response.text().await.expect("failed to read body");
    assert_full_page(&body);
}
