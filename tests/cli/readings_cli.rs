use crate::helpers::{create_author, create_book, create_reading, create_token, run_booklog};
use crate::test_macros::{define_cli_auth_test, define_cli_list_test};
use serde_json::Value;

define_cli_auth_test!(
    test_add_reading_requires_authentication,
    &["reading", "add", "--book-id", "123"]
);
define_cli_auth_test!(
    test_update_reading_requires_authentication,
    &["reading", "update", "--id", "123", "--status", "reading"]
);
define_cli_auth_test!(
    test_delete_reading_requires_authentication,
    &["reading", "delete", "--id", "123"]
);
define_cli_list_test!(
    test_list_readings_works_without_authentication,
    &["reading", "list"]
);

#[test]
fn test_add_reading_with_authentication() {
    let token = create_token("test-add-reading");

    let author_id = create_author("Reading Add Author", &token);
    let book_id = create_book("Reading Add Book", &author_id, &token);

    let output = run_booklog(
        &[
            "reading",
            "add",
            "--book-id",
            &book_id,
            "--status",
            "reading",
            "--format",
            "physical",
            "--started-at",
            "2025-01-01",
        ],
        &[("BOOKLOG_TOKEN", &token)],
    );

    assert!(
        output.status.success(),
        "reading add should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let reading: Value = serde_json::from_str(&stdout).expect("Should output valid JSON");

    assert_eq!(reading["status"], "reading");
    assert_eq!(reading["format"], "physical");
    assert!(reading["id"].is_i64());
}

#[test]
fn test_list_readings_shows_added_reading() {
    let token = create_token("test-list-readings");

    let author_id = create_author("Reading List Author", &token);
    let book_id = create_book("Reading List Book", &author_id, &token);
    let reading_id = create_reading(&book_id, "reading", &token);

    let output = run_booklog(&["reading", "list"], &[("BOOKLOG_TOKEN", &token)]);

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let readings: Value = serde_json::from_str(&stdout).unwrap();
    assert!(readings.is_array());
    let readings_array = readings.as_array().unwrap();
    assert!(
        readings_array
            .iter()
            .any(|r| r["id"].as_i64().map(|id| id.to_string()) == Some(reading_id.clone()))
    );
}

#[test]
fn test_update_reading_with_authentication() {
    let token = create_token("test-update-reading");

    let author_id = create_author("Reading Update Author", &token);
    let book_id = create_book("Reading Update Book", &author_id, &token);
    let reading_id = create_reading(&book_id, "reading", &token);

    let output = run_booklog(
        &[
            "reading",
            "update",
            "--id",
            &reading_id,
            "--status",
            "read",
            "--format",
            "ereader",
        ],
        &[("BOOKLOG_TOKEN", &token)],
    );

    assert!(
        output.status.success(),
        "reading update should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let reading: Value = serde_json::from_str(&stdout).expect("Should output valid JSON");

    assert_eq!(reading["status"], "read");
    assert_eq!(reading["format"], "ereader");
}

#[test]
fn test_delete_reading_with_authentication() {
    let token = create_token("test-delete-reading");

    let author_id = create_author("Reading Delete Author", &token);
    let book_id = create_book("Reading Delete Book", &author_id, &token);
    let reading_id = create_reading(&book_id, "reading", &token);

    let output = run_booklog(
        &["reading", "delete", "--id", &reading_id],
        &[("BOOKLOG_TOKEN", &token)],
    );
    assert!(output.status.success());

    let get_output = run_booklog(&["reading", "get", "--id", &reading_id], &[]);
    assert!(!get_output.status.success());
}
