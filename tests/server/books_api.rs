use crate::helpers::{
    create_default_author, create_default_book, create_default_reading, create_genre_with_name,
    spawn_app_with_auth,
};
use crate::test_macros::define_crud_tests;
use booklog::domain::book_items::{AuthorRole, BookAuthor, BookWithAuthors, NewBook, UpdateBook};
use booklog::domain::ids::AuthorId;

define_crud_tests!(
    entity: book,
    path: "/books",
    list_type: BookWithAuthors,
    malformed_json: r#"{"title": "Test", "authors": }"#,
    missing_fields: r#"{}"#
);

#[tokio::test]
async fn creating_a_book_returns_a_201_for_valid_data() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let client = reqwest::Client::new();

    let new_book = NewBook {
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
        primary_genre_id: None,
        secondary_genre_id: None,
        created_at: None,
    };

    let response = client
        .post(app.api_url("/books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_book)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 201);

    let book: booklog::domain::book_items::Book =
        response.json().await.expect("Failed to parse response");
    assert_eq!(book.title, "The Left Hand of Darkness");
    assert_eq!(book.isbn, Some("978-0441478125".to_string()));
    assert_eq!(book.page_count, Some(304));
    assert_eq!(book.year_published, Some(1969));
}

#[tokio::test]
async fn creating_a_book_persists_the_data() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let client = reqwest::Client::new();

    let new_book = NewBook {
        title: "Persistent Book".to_string(),
        authors: vec![BookAuthor {
            author_id: author.id,
            role: AuthorRole::default(),
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
    };

    let response = client
        .post(app.api_url("/books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_book)
        .send()
        .await
        .expect("Failed to execute request");

    let book: booklog::domain::book_items::Book =
        response.json().await.expect("Failed to parse response");

    let fetched_book = app
        .book_repo
        .get(book.id)
        .await
        .expect("Failed to fetch book");

    assert_eq!(fetched_book.title, "Persistent Book");
}

#[tokio::test]
async fn creating_a_book_with_nonexistent_author_returns_a_404() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let new_book = NewBook {
        title: "Orphaned Book".to_string(),
        authors: vec![BookAuthor {
            author_id: AuthorId::new(999999),
            role: AuthorRole::default(),
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
    };

    let response = client
        .post(app.api_url("/books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_book)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn getting_a_book_returns_a_200_for_valid_id() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.api_url(&format!("/books/{}", book.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn listing_books_returns_a_200_with_multiple_books() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let client = reqwest::Client::new();

    let book1 = NewBook {
        title: "First Book".to_string(),
        authors: vec![BookAuthor {
            author_id: author.id,
            role: AuthorRole::default(),
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
    };

    let book2 = NewBook {
        title: "Second Book".to_string(),
        authors: vec![BookAuthor {
            author_id: author.id,
            role: AuthorRole::default(),
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
    };

    client
        .post(app.api_url("/books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&book1)
        .send()
        .await
        .expect("Failed to create first book");

    client
        .post(app.api_url("/books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&book2)
        .send()
        .await
        .expect("Failed to create second book");

    let response = client
        .get(app.api_url("/books"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let books: Vec<BookWithAuthors> = response.json().await.expect("Failed to parse response");
    assert_eq!(books.len(), 2);
}

#[tokio::test]
async fn deleting_a_book_returns_a_204_for_valid_id() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    let response = client
        .delete(app.api_url(&format!("/books/{}", book.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 204);

    let get_response = client
        .get(app.api_url(&format!("/books/{}", book.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(get_response.status(), 404);
}

// --- Normalization tests ---

#[tokio::test]
async fn creating_a_book_trims_whitespace_from_fields() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let client = reqwest::Client::new();

    let new_book = NewBook {
        title: "  Trimmed Title  ".to_string(),
        authors: vec![BookAuthor {
            author_id: author.id,
            role: AuthorRole::default(),
        }],
        isbn: Some("  978-1234567890  ".to_string()),
        description: Some("  A description  ".to_string()),
        page_count: Some(200),
        year_published: None,
        publisher: Some("  Publisher Name  ".to_string()),
        language: Some("  English  ".to_string()),
        primary_genre_id: None,
        secondary_genre_id: None,
        created_at: None,
    };

    let response = client
        .post(app.api_url("/books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_book)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 201);

    let book: booklog::domain::book_items::Book =
        response.json().await.expect("Failed to parse response");
    assert_eq!(book.title, "Trimmed Title");
    assert_eq!(book.isbn, Some("978-1234567890".to_string()));
    assert_eq!(book.publisher, Some("Publisher Name".to_string()));
    assert_eq!(book.language, Some("English".to_string()));
}

#[tokio::test]
async fn creating_a_book_filters_out_zero_page_count() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let client = reqwest::Client::new();

    let new_book = NewBook {
        title: "Zero Pages Book".to_string(),
        authors: vec![BookAuthor {
            author_id: author.id,
            role: AuthorRole::default(),
        }],
        isbn: None,
        description: None,
        page_count: Some(0),
        year_published: None,
        publisher: None,
        language: None,
        primary_genre_id: None,
        secondary_genre_id: None,
        created_at: None,
    };

    let response = client
        .post(app.api_url("/books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_book)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 201);

    let book: booklog::domain::book_items::Book =
        response.json().await.expect("Failed to parse response");
    assert_eq!(book.page_count, None);
}

#[tokio::test]
async fn updating_a_book_normalizes_fields() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    let update = UpdateBook {
        publisher: Some("  Updated Publisher  ".to_string()),
        page_count: Some(-5),
        ..Default::default()
    };

    let response = client
        .put(app.api_url(&format!("/books/{}", book.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&update)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let updated: BookWithAuthors = response.json().await.expect("Failed to parse response");
    assert_eq!(
        updated.book.publisher,
        Some("Updated Publisher".to_string())
    );
    // Negative page count should be filtered out
    assert_eq!(updated.book.page_count, None);
}

// --- Relationship tests ---

#[tokio::test]
async fn deleting_a_book_cascades_to_readings() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let reading = create_default_reading(&app, book.id).await;
    let client = reqwest::Client::new();

    // Delete the book
    let response = client
        .delete(app.api_url(&format!("/books/{}", book.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 204);

    // The reading should also be gone
    let reading_response = client
        .get(app.api_url(&format!("/readings/{}", reading.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(reading_response.status(), 404);
}

#[tokio::test]
async fn creating_a_book_with_duplicate_title_returns_409() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let client = reqwest::Client::new();

    let new_book = NewBook {
        title: "Duplicate Title".to_string(),
        authors: vec![BookAuthor {
            author_id: author.id,
            role: AuthorRole::default(),
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
    };

    // First create succeeds
    let response = client
        .post(app.api_url("/books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_book)
        .send()
        .await
        .expect("Failed to create first book");

    assert_eq!(response.status(), 201);

    // Second create with same title should fail
    let response = client
        .post(app.api_url("/books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_book)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 409);
}

#[tokio::test]
async fn updating_a_book_with_no_changes_returns_400() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let client = reqwest::Client::new();

    let update = UpdateBook::default();

    let response = client
        .put(app.api_url(&format!("/books/{}", book.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&update)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn deleting_a_genre_clears_genre_references_on_books() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let genre = create_genre_with_name(&app, "Mystery").await;
    let client = reqwest::Client::new();

    let new_book = NewBook {
        title: "Genre Test Book".to_string(),
        authors: vec![BookAuthor {
            author_id: author.id,
            role: AuthorRole::default(),
        }],
        isbn: None,
        description: None,
        page_count: None,
        year_published: None,
        publisher: None,
        language: None,
        primary_genre_id: Some(genre.id),
        secondary_genre_id: None,
        created_at: None,
    };

    let book: BookWithAuthors = client
        .post(app.api_url("/books"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_book)
        .send()
        .await
        .expect("Failed to create book")
        .json()
        .await
        .expect("Failed to parse response");

    assert_eq!(book.book.primary_genre_id, Some(genre.id));

    // Delete the genre
    let response = client
        .delete(app.api_url(&format!("/genres/{}", genre.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to delete genre");
    assert_eq!(response.status(), 204);

    // Fetch the book — genre reference should be cleared
    let response = client
        .get(app.api_url(&format!("/books/{}", book.book.id)))
        .send()
        .await
        .expect("Failed to fetch book");
    assert_eq!(response.status(), 200);

    let updated_book: BookWithAuthors = response.json().await.expect("Failed to parse response");
    assert_eq!(updated_book.book.primary_genre_id, None);
}
