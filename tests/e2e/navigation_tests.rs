use thirtyfour::prelude::*;

use crate::helpers::auth::authenticate_browser;
use crate::helpers::browser::BrowserSession;
use crate::helpers::server_helpers::{
    create_default_author, create_default_book, spawn_app_with_auth,
};
use crate::helpers::wait::wait_for_visible;

#[tokio::test]
async fn home_page_shows_quick_actions_and_data() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let _book = create_default_book(&app, author.id).await;

    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    session.goto("/").await.unwrap();
    wait_for_visible(&session.driver, "body").await.unwrap();

    let body = session.driver.find(By::Css("body")).await.unwrap();
    let text = body.text().await.unwrap();

    // Quick action links should be present for authenticated users
    assert!(text.contains("Add"), "Should show Add action");

    session.quit().await;
}

#[tokio::test]
async fn stats_page_renders_with_data() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let _book = create_default_book(&app, author.id).await;

    // Stats are computed by a background task with debouncing.
    // Give it time to populate.
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    session.goto("/stats").await.unwrap();
    wait_for_visible(&session.driver, "h1").await.unwrap();

    let body = session.driver.find(By::Css("body")).await.unwrap();
    let text = body.text().await.unwrap();

    assert!(text.contains("Stats"), "Should show Stats heading");

    session.quit().await;
}

#[tokio::test]
async fn timeline_page_loads_events() {
    let app = spawn_app_with_auth().await;
    let _author = create_default_author(&app).await;

    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    session.goto("/timeline").await.unwrap();
    wait_for_visible(&session.driver, "#timeline-events")
        .await
        .unwrap();

    let body = session.driver.find(By::Css("body")).await.unwrap();
    let text = body.text().await.unwrap();
    // Creating an author generates a timeline event
    assert!(
        text.contains("Test Author"),
        "Should show author name in timeline"
    );

    session.quit().await;
}
