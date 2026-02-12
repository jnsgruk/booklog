use crate::helpers::{
    TestApp, create_default_author, create_default_book, create_default_genre,
    create_default_reading, spawn_app_with_auth,
};
use crate::test_macros::{define_form_create_tests, define_form_update_tests};

// ---------------------------------------------------------------------------
// Setup functions: create prerequisites + return form fields
// ---------------------------------------------------------------------------

async fn author_form_fields(_app: &TestApp) -> Vec<(String, String)> {
    vec![("name".into(), "Form Author".into())]
}

async fn genre_form_fields(_app: &TestApp) -> Vec<(String, String)> {
    vec![("name".into(), "Form Genre".into())]
}

async fn book_form_fields(app: &TestApp) -> Vec<(String, String)> {
    let author = create_default_author(app).await;
    vec![
        ("title".into(), "Form Book".into()),
        ("author_id".into(), author.id.into_inner().to_string()),
    ]
}

async fn reading_form_fields(app: &TestApp) -> Vec<(String, String)> {
    let author = create_default_author(app).await;
    let book = create_default_book(app, author.id).await;
    vec![
        ("book_id".into(), book.id.into_inner().to_string()),
        ("status".into(), "reading".into()),
        ("format".into(), "physical".into()),
        ("started_at".into(), "2025-01-01".into()),
    ]
}

// ---------------------------------------------------------------------------
// Update setup functions: create entity + return (id, update form fields)
// ---------------------------------------------------------------------------

async fn author_update_form(app: &TestApp) -> (String, Vec<(String, String)>) {
    let author = create_default_author(app).await;
    (
        author.id.into_inner().to_string(),
        vec![("name".into(), "Updated Author".into())],
    )
}

async fn genre_update_form(app: &TestApp) -> (String, Vec<(String, String)>) {
    let genre = create_default_genre(app).await;
    (
        genre.id.into_inner().to_string(),
        vec![("name".into(), "Updated Genre".into())],
    )
}

async fn book_update_form(app: &TestApp) -> (String, Vec<(String, String)>) {
    let author = create_default_author(app).await;
    let book = create_default_book(app, author.id).await;
    (
        book.id.into_inner().to_string(),
        vec![("title".into(), "Updated Book Title".into())],
    )
}

async fn reading_update_form(app: &TestApp) -> (String, Vec<(String, String)>) {
    let author = create_default_author(app).await;
    let book = create_default_book(app, author.id).await;
    let reading = create_default_reading(app, book.id).await;
    (
        reading.id.into_inner().to_string(),
        vec![
            ("status".into(), "read".into()),
            ("format".into(), "ereader".into()),
        ],
    )
}

// ---------------------------------------------------------------------------
// Macro-generated tests: form create -> 303 redirect + datastar variant
// ---------------------------------------------------------------------------

define_form_create_tests!(
    entity: author,
    api_path: "/authors",
    redirect_prefix: "/authors/",
    setup_and_form: author_form_fields
);

define_form_create_tests!(
    entity: genre,
    api_path: "/genres",
    redirect_prefix: "/genres/",
    setup_and_form: genre_form_fields
);

define_form_create_tests!(
    entity: book,
    api_path: "/books",
    redirect_prefix: "/books/",
    setup_and_form: book_form_fields
);

define_form_create_tests!(
    entity: reading,
    api_path: "/readings",
    redirect_prefix: "/readings/",
    setup_and_form: reading_form_fields
);

// ---------------------------------------------------------------------------
// Macro-generated tests: form update -> 303 redirect + datastar variant
// ---------------------------------------------------------------------------

define_form_update_tests!(
    entity: author,
    api_path: "/authors",
    redirect_prefix: "/authors/",
    setup_and_form: author_update_form
);

define_form_update_tests!(
    entity: genre,
    api_path: "/genres",
    redirect_prefix: "/genres/",
    setup_and_form: genre_update_form
);

define_form_update_tests!(
    entity: book,
    api_path: "/books",
    redirect_prefix: "/books/",
    setup_and_form: book_update_form
);

define_form_update_tests!(
    entity: reading,
    api_path: "/readings",
    redirect_prefix: "/readings/",
    setup_and_form: reading_update_form
);

// ---------------------------------------------------------------------------
// Hand-written tests: form-specific parsing edge cases
// ---------------------------------------------------------------------------

#[tokio::test]
async fn book_form_with_genre_ids() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let genre = create_default_genre(&app).await;

    let form_fields = vec![
        ("title", "Genre Test Book".to_string()),
        ("author_id", author.id.into_inner().to_string()),
        ("primary_genre_id", genre.id.into_inner().to_string()),
    ];

    let response = crate::helpers::post_form(&app, "/books", &form_fields).await;
    assert_eq!(response.status(), 303);

    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .expect("missing Location header");
    assert!(location.starts_with("/books/"));
}

#[tokio::test]
async fn book_form_with_genre_name_creates_genre() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;

    let form_fields = vec![
        ("title", "Genre Name Test Book".to_string()),
        ("author_id", author.id.into_inner().to_string()),
        ("primary_genre_id_name", "New Auto Genre".to_string()),
    ];

    let response = crate::helpers::post_form(&app, "/books", &form_fields).await;
    assert_eq!(response.status(), 303);

    // Verify genre was auto-created
    let client = reqwest::Client::new();
    let genres_response = client
        .get(app.api_url("/genres"))
        .send()
        .await
        .expect("Failed to list genres");
    let genres: Vec<serde_json::Value> = genres_response.json().await.unwrap();
    assert!(
        genres.iter().any(|g| g["name"] == "New Auto Genre"),
        "Genre should be auto-created from form genre name field"
    );
}

#[tokio::test]
async fn book_form_with_author_id_links_author() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;

    let form_fields = vec![
        ("title", "Author Link Test".to_string()),
        ("author_id", author.id.into_inner().to_string()),
    ];

    let response = crate::helpers::post_form(&app, "/books", &form_fields).await;
    assert_eq!(response.status(), 303);

    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .expect("missing Location header");
    let book_id: i64 = location.trim_start_matches("/books/").parse().unwrap();

    let client = reqwest::Client::new();
    let get_response = client
        .get(app.api_url(&format!("/books/{book_id}")))
        .send()
        .await
        .expect("Failed to get book");

    let book: booklog::domain::book_items::BookWithAuthors =
        get_response.json().await.expect("Failed to parse book");
    assert_eq!(book.authors.len(), 1, "book should have one author");
    assert_eq!(book.authors[0].author_id, author.id);
}

#[tokio::test]
async fn reading_form_with_empty_started_at() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;

    let form_fields = vec![
        ("book_id", book.id.into_inner().to_string()),
        ("status", "reading".into()),
        ("format", "physical".into()),
        ("started_at", String::new()), // empty string -> None
    ];

    let response = crate::helpers::post_form(&app, "/readings", &form_fields).await;
    assert_eq!(response.status(), 303);
}

// ---------------------------------------------------------------------------
// Form validation error tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn author_form_missing_name_returns_error() {
    let app = spawn_app_with_auth().await;
    let form_fields: Vec<(String, String)> = vec![];
    let response = crate::helpers::post_form(&app, "/authors", &form_fields).await;
    assert!(
        response.status().is_client_error(),
        "empty author form should fail, got {}",
        response.status()
    );
}

#[tokio::test]
async fn book_form_missing_title_returns_error() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;

    let form_fields = vec![("author_id", author.id.into_inner().to_string())];
    let response = crate::helpers::post_form(&app, "/books", &form_fields).await;
    assert!(
        response.status().is_client_error(),
        "book form without title should fail, got {}",
        response.status()
    );
}

#[tokio::test]
async fn reading_form_missing_book_id_returns_error() {
    let app = spawn_app_with_auth().await;

    let form_fields = vec![("status", "reading".to_string())];
    let response = crate::helpers::post_form(&app, "/readings", &form_fields).await;
    assert!(
        response.status().is_client_error(),
        "reading form without book_id should fail, got {}",
        response.status()
    );
}

#[tokio::test]
async fn reading_form_invalid_status_defaults_gracefully() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;

    // Invalid status values are silently defaulted (graceful degradation for form input)
    let form_fields = vec![
        ("book_id", book.id.into_inner().to_string()),
        ("status", "invalid_status".into()),
    ];
    let response = crate::helpers::post_form(&app, "/readings", &form_fields).await;
    assert_eq!(
        response.status(),
        303,
        "reading form with invalid status should succeed with default, got {}",
        response.status()
    );
}
