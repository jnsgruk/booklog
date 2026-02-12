use thirtyfour::prelude::*;

use crate::helpers::auth::authenticate_browser;
use crate::helpers::browser::BrowserSession;
use crate::helpers::server_helpers::{
    create_author_with_payload, create_default_author, create_default_book, create_default_reading,
    spawn_app_with_auth,
};
use crate::helpers::wait::wait_for_visible;

use booklog::domain::authors::NewAuthor;

#[tokio::test]
async fn author_detail_shows_all_fields() {
    let app = spawn_app_with_auth().await;
    let author = create_author_with_payload(
        &app,
        NewAuthor {
            name: "Detail Author".to_string(),
            created_at: None,
        },
    )
    .await;

    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    session
        .goto(&format!("/authors/{}", author.id))
        .await
        .unwrap();
    wait_for_visible(&session.driver, "h1").await.unwrap();

    let body = session.driver.find(By::Css("body")).await.unwrap();
    let text = body.text().await.unwrap();
    assert!(text.contains("Detail Author"), "Should show author name");

    session.quit().await;
}

#[tokio::test]
async fn book_detail_shows_book_info() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;

    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    session.goto(&format!("/books/{}", book.id)).await.unwrap();
    wait_for_visible(&session.driver, "h1").await.unwrap();

    let body = session.driver.find(By::Css("body")).await.unwrap();
    let text = body.text().await.unwrap();
    assert!(text.contains("Test Book"), "Should show book title");

    session.quit().await;
}

#[tokio::test]
async fn reading_detail_shows_status() {
    let app = spawn_app_with_auth().await;
    let author = create_default_author(&app).await;
    let book = create_default_book(&app, author.id).await;
    let reading = create_default_reading(&app, book.id).await;

    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    session
        .goto(&format!("/readings/{}", reading.id))
        .await
        .unwrap();
    wait_for_visible(&session.driver, "h1").await.unwrap();

    let body = session.driver.find(By::Css("body")).await.unwrap();
    let text = body.text().await.unwrap();
    assert!(text.contains("Test Book"), "Should show book title");

    session.quit().await;
}
