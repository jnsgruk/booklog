use thirtyfour::prelude::*;

use crate::helpers::auth::authenticate_browser;
use crate::helpers::browser::BrowserSession;
use crate::helpers::forms::{fill_input, submit_visible_form};
use crate::helpers::server_helpers::spawn_app_with_auth;
use crate::helpers::wait::{wait_for_url_contains, wait_for_visible};

#[tokio::test]
async fn create_author_via_add_page() {
    let app = spawn_app_with_auth().await;
    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    // Navigate to /add -- author tab is the default
    session.goto("/add").await.unwrap();

    // Wait for Datastar to initialize and show the author form
    wait_for_visible(&session.driver, "input[name='name']")
        .await
        .unwrap();

    // Fill the author form
    fill_input(&session.driver, "name", "E2E Test Author")
        .await
        .unwrap();

    // Submit -- standard method=post, redirects to detail page
    submit_visible_form(&session.driver).await.unwrap();

    // Should redirect to the author detail page
    wait_for_url_contains(&session.driver, "/authors/")
        .await
        .unwrap();

    // Verify the detail page shows the author name
    let heading = session.driver.find(By::Css("h1")).await.unwrap();
    let heading_text = heading.text().await.unwrap();
    assert_eq!(heading_text, "E2E Test Author");

    // Navigate to the data page and verify the author appears in the list
    session.goto("/data?type=authors").await.unwrap();

    let body = session.driver.find(By::Css("body")).await.unwrap();
    let body_text = body.text().await.unwrap();
    assert!(
        body_text.contains("E2E Test Author"),
        "Author should appear in the data list"
    );

    session.quit().await;
}

#[tokio::test]
async fn create_author_with_all_fields() {
    let app = spawn_app_with_auth().await;
    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    session.goto("/add").await.unwrap();

    wait_for_visible(&session.driver, "input[name='name']")
        .await
        .unwrap();

    fill_input(&session.driver, "name", "Full Detail Author")
        .await
        .unwrap();

    submit_visible_form(&session.driver).await.unwrap();

    wait_for_url_contains(&session.driver, "/authors/")
        .await
        .unwrap();

    // Verify detail page shows the author name
    let body = session.driver.find(By::Css("body")).await.unwrap();
    let body_text = body.text().await.unwrap();
    assert!(body_text.contains("Full Detail Author"));

    session.quit().await;
}
