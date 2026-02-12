use thirtyfour::prelude::*;

use crate::helpers::auth::authenticate_browser;
use crate::helpers::browser::BrowserSession;
use crate::helpers::forms::{select_option, select_searchable, submit_visible_form};
use crate::helpers::server_helpers::{
    create_default_author, create_default_book, spawn_app_with_auth,
};
use crate::helpers::wait::{wait_for_url_contains, wait_for_visible};

#[tokio::test]
async fn create_reading_with_api_prerequisites() {
    let app = spawn_app_with_auth().await;

    // Create prerequisite author and book via API
    let author = create_default_author(&app).await;
    let _book = create_default_book(&app, author.id).await;

    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    // Navigate to add page with reading tab
    session.goto("/add?type=reading").await.unwrap();

    // Wait for the reading form's book select to appear
    wait_for_visible(&session.driver, "searchable-select[name='book_id']")
        .await
        .unwrap();

    // Select the book
    select_searchable(&session.driver, "book_id", "Test Book")
        .await
        .unwrap();

    // Select status "reading" to reveal the format field (hidden when "on_shelf")
    select_option(&session.driver, "status", "reading")
        .await
        .unwrap();

    // Wait for Datastar to show the format field
    wait_for_visible(&session.driver, "select[name='format']")
        .await
        .unwrap();

    // Select the reading format
    select_option(&session.driver, "format", "physical")
        .await
        .unwrap();

    // Submit the form
    submit_visible_form(&session.driver).await.unwrap();

    // Should redirect to the reading detail page
    wait_for_url_contains(&session.driver, "/readings/")
        .await
        .unwrap();

    // Verify the reading detail page loaded
    let body = session.driver.find(By::Css("body")).await.unwrap();
    let body_text = body.text().await.unwrap();
    assert!(
        body_text.contains("Test Book"),
        "Reading detail should show the book title"
    );
    assert!(
        body_text.contains("Physical"),
        "Reading detail should show the format"
    );

    session.quit().await;
}
