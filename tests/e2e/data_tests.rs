use std::time::Duration;

use thirtyfour::prelude::*;

use crate::helpers::auth::authenticate_browser;
use crate::helpers::browser::BrowserSession;
use crate::helpers::server_helpers::{
    create_default_genre, create_library_item, spawn_app_with_auth,
};
use crate::helpers::wait::{wait_for_text, wait_for_visible};

#[tokio::test]
async fn tab_switching_loads_correct_entity_list() {
    let app = spawn_app_with_auth().await;
    create_library_item(&app, "Test Book").await;
    create_default_genre(&app).await;

    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    // Navigate to data page (defaults to Library tab)
    session.goto("/data").await.unwrap();
    wait_for_visible(&session.driver, "#data-content")
        .await
        .unwrap();

    // Verify library content shows the book
    wait_for_text(&session.driver, "#data-content", "Test Book")
        .await
        .unwrap();

    // Click "Genres" tab (desktop)
    let tabs = session
        .driver
        .find_all(By::Css("nav[role='tablist'] button[role='tab']"))
        .await
        .unwrap();
    for tab in &tabs {
        let text = tab.text().await.unwrap_or_default();
        if text.contains("Genres") {
            tab.click().await.unwrap();
            break;
        }
    }

    // Wait for genre list to load
    wait_for_text(&session.driver, "#data-content", "Test Genre")
        .await
        .unwrap();

    session.quit().await;
}

#[tokio::test]
async fn search_filters_list() {
    let app = spawn_app_with_auth().await;
    create_library_item(&app, "Pride and Prejudice").await;
    create_library_item(&app, "War and Peace").await;

    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    session.goto("/data").await.unwrap();
    wait_for_visible(&session.driver, "#user-book-list")
        .await
        .unwrap();

    // Both books should be visible initially
    let body = session.driver.find(By::Css("#data-content")).await.unwrap();
    let text = body.text().await.unwrap();
    assert!(
        text.contains("Pride and Prejudice"),
        "Should show Pride and Prejudice"
    );
    assert!(text.contains("War and Peace"), "Should show War and Peace");

    // Type in search field
    let search = session
        .driver
        .find(By::Css("input[type='search']"))
        .await
        .unwrap();
    search.send_keys("Pride").await.unwrap();

    // Wait for debounce (300ms) + fragment load
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Wait for the list to update with filtered results
    wait_for_text(&session.driver, "#data-content", "Pride and Prejudice")
        .await
        .unwrap();

    // War and Peace should be filtered out
    let text = session
        .driver
        .find(By::Css("#data-content"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        !text.contains("War and Peace"),
        "War and Peace should be filtered out"
    );

    session.quit().await;
}

#[tokio::test]
async fn pagination_next_and_prev() {
    let app = spawn_app_with_auth().await;

    // Create 11 library items -- default page size is 10
    for i in 0..11 {
        create_library_item(&app, &format!("Book {i:02}")).await;
    }

    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    session.goto("/data").await.unwrap();
    wait_for_visible(&session.driver, ".pagination-controls")
        .await
        .unwrap();

    // Should show "Page 1 of 2"
    let pagination = session
        .driver
        .find(By::Css(".pagination-controls"))
        .await
        .unwrap();
    let pagination_text = pagination.text().await.unwrap();
    assert!(
        pagination_text.contains("Page 1 of 2"),
        "Should show page 1 of 2, got: {pagination_text}"
    );

    // Click Next
    let next_btn = pagination
        .find(By::XPath(".//button[contains(., 'Next')]"))
        .await
        .unwrap();
    next_btn.click().await.unwrap();

    // Wait for page 2
    tokio::time::sleep(Duration::from_millis(300)).await;
    wait_for_text(&session.driver, ".pagination-controls", "Page 2 of 2")
        .await
        .unwrap();

    // Click Prev
    let pagination = session
        .driver
        .find(By::Css(".pagination-controls"))
        .await
        .unwrap();
    let prev_btn = pagination
        .find(By::XPath(".//button[contains(., 'Prev')]"))
        .await
        .unwrap();
    prev_btn.click().await.unwrap();

    wait_for_text(&session.driver, ".pagination-controls", "Page 1 of 2")
        .await
        .unwrap();

    session.quit().await;
}

#[tokio::test]
async fn sort_by_column_reverses_order() {
    let app = spawn_app_with_auth().await;
    create_library_item(&app, "Alpha Book").await;
    create_library_item(&app, "Zulu Novel").await;

    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    session.goto("/data").await.unwrap();
    wait_for_visible(&session.driver, "#user-book-list")
        .await
        .unwrap();

    // Click "Title" sort header to sort ascending
    let title_header = session
        .driver
        .find(By::XPath("//thead//button[contains(., 'Title')]"))
        .await
        .unwrap();
    title_header.click().await.unwrap();

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Get row order after first click (ascending)
    let rows = session
        .driver
        .find_all(By::Css("#user-book-list tbody tr"))
        .await
        .unwrap();
    let mut first_order = Vec::new();
    for row in &rows {
        first_order.push(row.text().await.unwrap_or_default());
    }

    // Click again to reverse
    let title_header = session
        .driver
        .find(By::XPath("//thead//button[contains(., 'Title')]"))
        .await
        .unwrap();
    title_header.click().await.unwrap();

    tokio::time::sleep(Duration::from_millis(300)).await;

    let rows = session
        .driver
        .find_all(By::Css("#user-book-list tbody tr"))
        .await
        .unwrap();
    let mut second_order = Vec::new();
    for row in &rows {
        second_order.push(row.text().await.unwrap_or_default());
    }

    // Orders should be different (reversed)
    assert_ne!(
        first_order, second_order,
        "Clicking sort header twice should reverse order"
    );

    session.quit().await;
}
