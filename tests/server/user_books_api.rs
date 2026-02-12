use crate::helpers::{create_default_author, create_default_book, spawn_app_with_auth};
use booklog::domain::user_books::UserBook;
use reqwest::Client;
use serde_json::json;

// --- CRUD ---

#[tokio::test]
async fn creating_a_user_book_returns_201() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = Client::new();

    let response = client
        .post(app.api_url("/user-books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&json!({"book_id": book.id, "shelf": "library"}))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 201);

    let user_book: UserBook = response.json().await.expect("Failed to parse response");
    assert_eq!(user_book.book_id, book.id);
    assert_eq!(user_book.shelf.as_str(), "library");
    assert!(!user_book.book_club);
}

#[tokio::test]
async fn creating_a_user_book_with_wishlist_shelf() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = Client::new();

    let response = client
        .post(app.api_url("/user-books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&json!({"book_id": book.id, "shelf": "wishlist", "book_club": true}))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 201);

    let user_book: UserBook = response.json().await.expect("Failed to parse response");
    assert_eq!(user_book.shelf.as_str(), "wishlist");
    assert!(user_book.book_club);
}

#[tokio::test]
async fn listing_user_books_returns_created_items() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = Client::new();

    // Create a user book
    client
        .post(app.api_url("/user-books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&json!({"book_id": book.id}))
        .send()
        .await
        .expect("Failed to create user book");

    // List user books
    let response = client
        .get(app.api_url("/user-books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let user_books: Vec<UserBook> = response.json().await.expect("Failed to parse response");
    assert_eq!(user_books.len(), 1);
    assert_eq!(user_books[0].book_id, book.id);
}

#[tokio::test]
async fn moving_a_user_book_shelf() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = Client::new();

    let created: UserBook = client
        .post(app.api_url("/user-books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&json!({"book_id": book.id, "shelf": "library"}))
        .send()
        .await
        .expect("Failed to create user book")
        .json()
        .await
        .expect("Failed to parse response");

    assert_eq!(created.shelf.as_str(), "library");

    let response = client
        .put(app.api_url(&format!("/user-books/{}", created.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&json!({"shelf": "wishlist"}))
        .send()
        .await
        .expect("Failed to move shelf");

    assert_eq!(response.status(), 200);

    let moved: UserBook = response.json().await.expect("Failed to parse response");
    assert_eq!(moved.shelf.as_str(), "wishlist");
}

#[tokio::test]
async fn setting_book_club_flag() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = Client::new();

    let created: UserBook = client
        .post(app.api_url("/user-books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&json!({"book_id": book.id}))
        .send()
        .await
        .expect("Failed to create user book")
        .json()
        .await
        .expect("Failed to parse response");

    assert!(!created.book_club);

    let response = client
        .patch(app.api_url(&format!("/user-books/{}", created.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&json!({"book_club": true}))
        .send()
        .await
        .expect("Failed to set book club");

    assert_eq!(response.status(), 200);

    let updated: UserBook = response.json().await.expect("Failed to parse response");
    assert!(updated.book_club);
}

#[tokio::test]
async fn deleting_a_user_book() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = Client::new();

    let created: UserBook = client
        .post(app.api_url("/user-books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&json!({"book_id": book.id}))
        .send()
        .await
        .expect("Failed to create user book")
        .json()
        .await
        .expect("Failed to parse response");

    let response = client
        .delete(app.api_url(&format!("/user-books/{}", created.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to delete user book");

    assert_eq!(response.status(), 204);

    // Verify it's gone by listing
    let list_response = client
        .get(app.api_url("/user-books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to list user books");

    let user_books: Vec<UserBook> = list_response.json().await.expect("Failed to parse");
    assert_eq!(user_books.len(), 0);
}

// --- Authorization ---

#[tokio::test]
async fn creating_a_user_book_requires_auth() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = Client::new();

    let response = client
        .post(app.api_url("/user-books"))
        .json(&json!({"book_id": book.id}))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn deleting_a_nonexistent_user_book_returns_404() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .delete(app.api_url("/user-books/999999"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn duplicate_user_book_returns_409() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = Client::new();

    let payload = json!({"book_id": book.id});

    let response = client
        .post(app.api_url("/user-books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to create first user book");
    assert_eq!(response.status(), 201);

    let response = client
        .post(app.api_url("/user-books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("Failed to create duplicate user book");
    assert_eq!(response.status(), 409);
}

// --- Cascade deletes ---

#[tokio::test]
async fn deleting_a_book_cascades_to_user_books() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = Client::new();

    // Create a user book
    client
        .post(app.api_url("/user-books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&json!({"book_id": book.id}))
        .send()
        .await
        .expect("Failed to create user book");

    // Delete the book
    let response = client
        .delete(app.api_url(&format!("/books/{}", book.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to delete book");
    assert_eq!(response.status(), 204);

    // User book list should be empty
    let list_response = client
        .get(app.api_url("/user-books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to list user books");

    let user_books: Vec<UserBook> = list_response.json().await.expect("Failed to parse");
    assert_eq!(user_books.len(), 0);
}
