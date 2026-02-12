use crate::helpers::{create_genre, create_token, run_booklog};
use crate::test_macros::{define_cli_auth_test, define_cli_list_test};
use serde_json::Value;

define_cli_auth_test!(
    test_add_genre_requires_authentication,
    &["genre", "add", "--name", "Test Genre"]
);
define_cli_auth_test!(
    test_delete_genre_requires_authentication,
    &["genre", "delete", "--id", "some-id"]
);
define_cli_auth_test!(
    test_update_genre_requires_authentication,
    &[
        "genre",
        "update",
        "--id",
        "some-id",
        "--name",
        "Updated Name"
    ]
);
define_cli_list_test!(
    test_list_genres_works_without_authentication,
    &["genre", "list"]
);

#[test]
fn test_add_genre_with_authentication() {
    let token = create_token("test-add-genre");

    let output = run_booklog(
        &["genre", "add", "--name", "Test Genre"],
        &[("BOOKLOG_TOKEN", &token)],
    );

    assert!(
        output.status.success(),
        "genre add with auth should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let genre: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("Should output valid JSON, got: {}", stdout));

    assert_eq!(genre["name"], "Test Genre");
    assert!(genre["id"].is_i64(), "Should have an ID");
}

#[test]
fn test_list_genres_shows_added_genre() {
    let token = create_token("test-list-genres");

    let genre_id = create_genre("Example Genre", &token);

    let list_output = run_booklog(&["genre", "list"], &[]);

    assert!(list_output.status.success());

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    let genres: Value = serde_json::from_str(&list_stdout).expect("Should output valid JSON array");

    assert!(genres.is_array(), "Should return an array");
    let genres_array = genres.as_array().unwrap();

    let found = genres_array
        .iter()
        .any(|g| g["id"].as_i64().unwrap().to_string() == genre_id);
    assert!(found, "Should find the added genre in the list");
}
