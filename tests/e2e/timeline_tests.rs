use tokio::time::{Duration, sleep};

use crate::helpers::auth::authenticate_browser;
use crate::helpers::browser::BrowserSession;
use crate::helpers::forms::{fill_input, submit_visible_form};
use crate::helpers::server_helpers::{create_author_with_payload, spawn_app_with_timeline_sync};
use crate::helpers::wait::{wait_for_url_not_contains, wait_for_visible};

use booklog::domain::authors::NewAuthor;

#[tokio::test]
async fn editing_an_author_updates_timeline_in_browser() {
    let app = spawn_app_with_timeline_sync().await;

    let original_name = "Original Browser Author";
    let author = create_author_with_payload(
        &app,
        NewAuthor {
            name: original_name.to_string(),
            created_at: None,
        },
    )
    .await;

    let session = BrowserSession::new(&app.address).await.unwrap();
    authenticate_browser(&session, &app).await.unwrap();

    // Verify the original name appears on the timeline page
    session.goto("/timeline").await.unwrap();
    wait_for_visible(&session.driver, "[data-timeline-event]")
        .await
        .unwrap();

    let body = session.driver.source().await.unwrap();
    assert!(
        body.contains(original_name),
        "Expected original author name on timeline page"
    );

    // Navigate to the author edit page and change the name
    session
        .goto(&format!("/authors/{}/edit", author.id))
        .await
        .unwrap();
    wait_for_visible(&session.driver, "input[name='name']")
        .await
        .unwrap();

    let updated_name = "Updated Browser Author";
    fill_input(&session.driver, "name", updated_name)
        .await
        .unwrap();
    submit_visible_form(&session.driver).await.unwrap();

    // Wait for redirect away from edit page
    wait_for_url_not_contains(&session.driver, "/edit")
        .await
        .unwrap();

    // Wait for debounce + processing
    sleep(Duration::from_millis(300)).await;

    // Navigate back to timeline and verify the updated name
    session.goto("/timeline").await.unwrap();
    wait_for_visible(&session.driver, "[data-timeline-event]")
        .await
        .unwrap();

    let body = session.driver.source().await.unwrap();
    assert!(
        body.contains(updated_name),
        "Expected updated author name on timeline page, got: {body}"
    );
    assert!(
        !body.contains(original_name),
        "Expected original author name to be replaced on timeline page"
    );

    session.quit().await;
}
