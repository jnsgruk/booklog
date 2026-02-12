use thirtyfour::prelude::*;

use crate::helpers::auth::authenticate_browser;
use crate::helpers::browser::BrowserSession;
use crate::helpers::server_helpers::{create_default_author, spawn_app_with_auth};
use crate::helpers::wait::{wait_for_url_contains, wait_for_visible};

#[tokio::test]
async fn delete_author_from_detail_page() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;

    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    // Navigate to the author detail page
    session
        .goto(&format!("/authors/{}", author.id))
        .await
        .unwrap();

    // Verify we're on the right page
    let heading = session.driver.find(By::Css("h1")).await.unwrap();
    assert_eq!(heading.text().await.unwrap(), "Test Author");

    // Click the delete button
    let delete_btn = wait_for_visible(&session.driver, "button.text-error")
        .await
        .unwrap();
    delete_btn.click().await.unwrap();

    // Accept the confirm() dialog
    session.driver.accept_alert().await.unwrap();

    // After @delete + redirect script, the browser navigates to /data
    wait_for_url_contains(&session.driver, "/data")
        .await
        .unwrap();

    // Verify the author is no longer in the list
    let body = session.driver.find(By::Css("body")).await.unwrap();
    let body_text = body.text().await.unwrap();
    assert!(
        !body_text.contains("Test Author"),
        "Deleted author should not appear in the list"
    );

    session.quit().await;
}
