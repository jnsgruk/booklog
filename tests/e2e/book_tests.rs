use thirtyfour::prelude::*;

use crate::helpers::auth::authenticate_browser;
use crate::helpers::browser::BrowserSession;
use crate::helpers::forms::{fill_input, select_searchable, submit_visible_form};
use crate::helpers::server_helpers::{create_default_author, spawn_app_with_auth};
use crate::helpers::wait::{wait_for_url_contains, wait_for_visible};

#[tokio::test]
async fn create_book_with_api_prerequisites() {
    let app = spawn_app_with_auth().await;

    // Create prerequisite author via API
    let _author = create_default_author(&app).await;

    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    // Navigate to add page with book tab
    session.goto("/add?type=book").await.unwrap();

    // Wait for the book form's author select to appear
    wait_for_visible(&session.driver, "searchable-select[name='author_id']")
        .await
        .unwrap();

    // Fill in the book title
    fill_input(&session.driver, "title", "Test Novel")
        .await
        .unwrap();

    // Select the author
    select_searchable(&session.driver, "author_id", "Test Author")
        .await
        .unwrap();

    // Submit the form
    submit_visible_form(&session.driver).await.unwrap();

    // Should redirect to the book detail page
    wait_for_url_contains(&session.driver, "/books/")
        .await
        .unwrap();

    // Verify the book detail page loaded
    let body = session.driver.find(By::Css("body")).await.unwrap();
    let body_text = body.text().await.unwrap();
    assert!(
        body_text.contains("Test Novel"),
        "Book detail should show the book title"
    );

    session.quit().await;
}
