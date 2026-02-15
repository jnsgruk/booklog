use std::time::Duration;

use crate::helpers::{create_author, create_book, create_token, run_booklog, server_info};
use crate::test_macros::define_cli_auth_test;

define_cli_auth_test!(
    test_timeline_rebuild_requires_authentication,
    &["timeline", "rebuild"]
);

/// Fetch the /timeline HTML page from the shared test server.
fn fetch_timeline_html() -> String {
    let (address, _) = server_info();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    rt.block_on(async {
        reqwest::get(format!("{address}/timeline"))
            .await
            .expect("failed to fetch timeline")
            .text()
            .await
            .expect("failed to read timeline body")
    })
}

#[test]
fn test_timeline_rebuild_updates_stale_events() {
    let token = create_token("test-timeline-rebuild");

    // Create an author and a book that references it
    let author_id = create_author("Original CLI Author", &token);
    let _book_id = create_book("CLI Book", &author_id, &token);

    // Small delay so timeline events are committed
    std::thread::sleep(Duration::from_millis(50));

    // Verify the original author name appears on the timeline
    let body = fetch_timeline_html();
    assert!(
        body.contains("Original CLI Author"),
        "Expected original author name in timeline before update"
    );

    // Update the author name via CLI
    let output = run_booklog(
        &[
            "author",
            "update",
            "--id",
            &author_id,
            "--name",
            "Updated CLI Author",
        ],
        &[("BOOKLOG_TOKEN", &token)],
    );
    assert!(
        output.status.success(),
        "author update should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Run timeline rebuild to refresh stale snapshots
    let output = run_booklog(&["timeline", "rebuild"], &[("BOOKLOG_TOKEN", &token)]);
    assert!(
        output.status.success(),
        "timeline rebuild should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Wait for debounce (100ms) + processing
    std::thread::sleep(Duration::from_millis(500));

    // Verify the timeline now shows the updated author name
    let body = fetch_timeline_html();
    assert!(
        body.contains("Updated CLI Author"),
        "Expected updated author name in timeline after rebuild, got: {body}"
    );
    assert!(
        !body.contains("Original CLI Author"),
        "Expected original author name to be replaced in timeline after rebuild"
    );
}
