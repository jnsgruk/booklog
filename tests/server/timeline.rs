use crate::helpers::{
    create_author_with_payload, create_default_author, create_default_book, spawn_app_with_auth,
};
use booklog::domain::authors::NewAuthor;
use booklog::domain::book_items::{AuthorRole, BookAuthor, NewBook};
use booklog::domain::ids::AuthorId;
use reqwest::Client;
use tokio::time::{Duration, sleep};

async fn create_book(app: &crate::helpers::TestApp, author_id: AuthorId, title: &str) {
    let client = Client::new();
    let book = NewBook {
        title: title.to_string(),
        authors: vec![BookAuthor {
            author_id,
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
        .json(&book)
        .send()
        .await
        .expect("failed to create book");

    assert_eq!(response.status(), 201);
}

async fn seed_timeline_with_books(
    app: &crate::helpers::TestApp,
    book_count: usize,
) -> (String, Vec<String>) {
    let author_name = "Timeline Seed Author";
    let author = create_author_with_payload(
        app,
        NewAuthor {
            name: author_name.to_string(),
            created_at: None,
        },
    )
    .await;

    // Ensure the author event predates the book events.
    sleep(Duration::from_millis(5)).await;

    let mut book_titles = Vec::new();
    for index in 0..book_count {
        let title = format!("Seed Book {index:02}");
        create_book(app, author.id, &title).await;
        book_titles.push(title);
        // Space out timestamps to keep ordering deterministic.
        sleep(Duration::from_millis(2)).await;
    }

    (author_name.to_string(), book_titles)
}

#[tokio::test]
async fn timeline_page_returns_a_200_with_empty_state() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .get(format!("{}/timeline", app.address))
        .send()
        .await
        .expect("failed to fetch timeline");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("failed to read response body");
    assert!(
        body.contains("No events yet"),
        "Expected empty timeline state message, got: {body}"
    );
}

#[tokio::test]
async fn creating_an_author_surfaces_on_the_timeline() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let author_name = "Timeline Author";
    create_author_with_payload(
        &app,
        NewAuthor {
            name: author_name.to_string(),
            created_at: None,
        },
    )
    .await;

    sleep(Duration::from_millis(10)).await;

    let response = client
        .get(format!("{}/timeline", app.address))
        .send()
        .await
        .expect("failed to fetch timeline");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("failed to read response body");
    assert!(
        body.contains("Author Added"),
        "Expected 'Author Added' badge in timeline HTML, got: {body}"
    );
    assert!(
        body.contains(author_name),
        "Expected author name to appear in timeline HTML, got: {body}"
    );
    assert!(
        body.contains("/authors/"),
        "Expected author detail link in timeline HTML, got: {body}"
    );
}

#[tokio::test]
async fn creating_a_book_surfaces_on_the_timeline() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let author_id = create_author_with_payload(
        &app,
        NewAuthor {
            name: "Timeline Book Author".to_string(),
            created_at: None,
        },
    )
    .await
    .id;

    sleep(Duration::from_millis(5)).await;
    let book_title = "Timeline Novel";
    create_book(&app, author_id, book_title).await;

    let response = client
        .get(format!("{}/timeline", app.address))
        .send()
        .await
        .expect("failed to fetch timeline");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("failed to read response body");
    assert!(
        body.contains("Book Added"),
        "Expected 'Book Added' badge in timeline HTML, got: {body}"
    );
    assert!(
        body.contains(book_title),
        "Expected book title to appear in timeline HTML, got: {body}"
    );
    // Genre is not set on this test book, so no genre assertion needed
}

#[tokio::test]
async fn timeline_page_signals_more_results_when_over_page_size() {
    let app = spawn_app_with_auth().await;
    let (_, book_titles) = seed_timeline_with_books(&app, 6).await;
    assert_eq!(book_titles.len(), 6);

    let client = Client::new();

    // Explicitly request page_size=5 to test pagination with 6 events
    let response = client
        .get(format!("{}/timeline?page_size=5", app.address))
        .send()
        .await
        .expect("failed to fetch timeline");

    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("failed to read response body");

    assert!(
        body.contains(
            "data-next-url=\"/timeline?page=2&#38;page_size=5&#38;sort=occurred-at&#38;dir=desc\""
        ),
        "Expected loader next-page URL missing from timeline HTML:\n{}",
        body
    );
    assert!(
        body.contains("data-has-more=\"true\""),
        "Expected loader to signal additional pages"
    );

    let latest_book = book_titles.last().unwrap();
    assert!(
        body.contains(latest_book),
        "Expected most recent book '{latest_book}' to appear in first page HTML"
    );

    let event_occurrences = body.matches("data-timeline-event").count();
    assert_eq!(
        event_occurrences, 5,
        "Expected exactly 5 events on first page"
    );
}

#[tokio::test]
async fn timeline_chunk_endpoint_serves_remaining_events() {
    let app = spawn_app_with_auth().await;
    let (author_name, book_titles) = seed_timeline_with_books(&app, 6).await;
    let oldest_book = book_titles
        .first()
        .expect("missing seeded book title")
        .clone();
    let client = Client::new();

    // page_size=5 to test pagination with 6 events
    let chunk_url = format!(
        "{}/timeline?page=2&page_size=5&sort=occurred-at&dir=desc",
        app.address
    );

    let response = client
        .get(chunk_url)
        .header("datastar-request", "true")
        .send()
        .await
        .expect("failed to fetch timeline chunk");

    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("failed to read response body");

    assert!(
        body.contains(&oldest_book),
        "Expected chunk payload to include oldest book '{oldest_book}':\n{}",
        body
    );
    assert!(
        body.contains(&author_name),
        "Expected chunk payload to include the author event: {body}"
    );
    assert!(
        body.contains("data-has-more=\"false\""),
        "Expected chunk to disable further pagination"
    );
    assert!(
        body.contains("data-next-url=\"\""),
        "Expected chunk to clear next URL once exhausted"
    );
}

#[tokio::test]
async fn creating_a_reading_surfaces_on_the_timeline() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;

    sleep(Duration::from_millis(10)).await;

    let reading_payload = serde_json::json!({
        "book_id": book.id,
        "status": "reading",
        "format": "physical",
        "started_at": "2025-01-01"
    });

    let response = client
        .post(app.api_url("/readings"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&reading_payload)
        .send()
        .await
        .expect("failed to create reading");
    assert_eq!(response.status(), 201);

    sleep(Duration::from_millis(10)).await;

    let response = client
        .get(format!("{}/timeline", app.address))
        .send()
        .await
        .expect("failed to fetch timeline");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("failed to read response body");
    assert!(
        body.contains("Started"),
        "Expected 'Started' badge in timeline HTML, got: {body}"
    );
    assert!(
        body.contains("Test Book"),
        "Expected book title to appear in reading timeline event, got: {body}"
    );
}

#[tokio::test]
async fn shelving_a_book_surfaces_on_the_timeline() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;

    sleep(Duration::from_millis(10)).await;

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
        .expect("failed to shelve book");
    assert_eq!(response.status(), 201);

    sleep(Duration::from_millis(10)).await;

    let response = client
        .get(format!("{}/timeline", app.address))
        .send()
        .await
        .expect("failed to fetch timeline");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("failed to read response body");
    assert!(
        body.contains("Shelved"),
        "Expected 'Shelved' badge in timeline HTML, got: {body}"
    );
    assert!(
        body.contains("Test Book"),
        "Expected book title in shelved timeline event, got: {body}"
    );
}
