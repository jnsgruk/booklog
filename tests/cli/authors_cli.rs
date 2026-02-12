use crate::helpers::{create_author, create_token, run_booklog};
use crate::test_macros::{define_cli_auth_test, define_cli_list_test};
use serde_json::Value;

define_cli_auth_test!(
    test_add_author_requires_authentication,
    &["author", "add", "--name", "Test Author"]
);
define_cli_auth_test!(
    test_delete_author_requires_authentication,
    &["author", "delete", "--id", "some-id"]
);
define_cli_auth_test!(
    test_update_author_requires_authentication,
    &[
        "author",
        "update",
        "--id",
        "some-id",
        "--name",
        "Updated Name"
    ]
);
define_cli_list_test!(
    test_list_authors_works_without_authentication,
    &["author", "list"]
);

#[test]
fn test_add_author_with_authentication() {
    let token = create_token("test-add-author");

    let output = run_booklog(
        &["author", "add", "--name", "Test Author"],
        &[("BOOKLOG_TOKEN", &token)],
    );

    assert!(
        output.status.success(),
        "author add with auth should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let author: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("Should output valid JSON, got: {}", stdout));

    assert_eq!(author["name"], "Test Author");
    assert!(author["id"].is_i64(), "Should have an ID");
}

#[test]
fn test_list_authors_shows_added_author() {
    let token = create_token("test-list-authors");

    let author_id = create_author("Example Author", &token);

    let list_output = run_booklog(&["author", "list"], &[]);

    assert!(list_output.status.success());

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    let authors: Value =
        serde_json::from_str(&list_stdout).expect("Should output valid JSON array");

    assert!(authors.is_array(), "Should return an array");
    let authors_array = authors.as_array().unwrap();

    let found = authors_array
        .iter()
        .any(|a| a["id"].as_i64().unwrap().to_string() == author_id);
    assert!(found, "Should find the added author in the list");
}
