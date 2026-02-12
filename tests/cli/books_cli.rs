use crate::helpers::{create_author, create_book, create_token, run_booklog};
use crate::test_macros::{define_cli_auth_test, define_cli_list_test};
use serde_json::Value;

define_cli_auth_test!(
    test_add_book_requires_authentication,
    &["book", "add", "--title", "Test Book"]
);
define_cli_auth_test!(
    test_update_book_requires_authentication,
    &["book", "update", "--id", "123", "--title", "Updated"]
);
define_cli_auth_test!(
    test_delete_book_requires_authentication,
    &["book", "delete", "--id", "some-id"]
);
define_cli_list_test!(
    test_list_books_works_without_authentication,
    &["book", "list"]
);

#[test]
fn test_add_book_with_authentication() {
    let token = create_token("test-add-book");

    let author_id = create_author("Book Add Author", &token);

    let output = run_booklog(
        &[
            "book",
            "add",
            "--title",
            "The Great Novel",
            "--author-ids",
            &author_id,
        ],
        &[("BOOKLOG_TOKEN", &token)],
    );

    assert!(
        output.status.success(),
        "book add with auth should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let book: Value = serde_json::from_str(&stdout).expect("Should output valid JSON");

    assert_eq!(book["title"], "The Great Novel");
    assert!(book["id"].is_i64(), "Should have an ID");
}

#[test]
fn test_list_books_shows_added_book() {
    let token = create_token("test-list-books");

    let author_id = create_author("Book List Author", &token);
    let book_id = create_book("Listable Novel", &author_id, &token);

    let list_output = run_booklog(&["book", "list"], &[]);

    assert!(list_output.status.success());

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    let books: Value = serde_json::from_str(&list_stdout).unwrap();

    assert!(books.is_array());
    let books_array = books.as_array().unwrap();

    let found = books_array
        .iter()
        .any(|item| item["id"].as_i64().map(|id| id.to_string()) == Some(book_id.clone()));
    assert!(found, "Should find the added book in the list");
}

#[test]
fn test_update_book_with_authentication() {
    let token = create_token("test-update-book");

    let author_id = create_author("Update Book Author", &token);
    let book_id = create_book("Original Title", &author_id, &token);

    let output = run_booklog(
        &[
            "book",
            "update",
            "--id",
            &book_id,
            "--title",
            "Updated Title",
        ],
        &[("BOOKLOG_TOKEN", &token)],
    );

    assert!(output.status.success());
    let updated_book: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(updated_book["title"], "Updated Title");
}
