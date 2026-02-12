use crate::helpers::{create_author, create_book, create_token, run_booklog};
use crate::test_macros::define_cli_auth_test;
use serde_json::Value;

define_cli_auth_test!(
    test_add_user_book_requires_authentication,
    &["user-book", "add", "--book-id", "1"]
);
define_cli_auth_test!(
    test_list_user_books_requires_authentication,
    &["user-book", "list"]
);
define_cli_auth_test!(
    test_move_user_book_requires_authentication,
    &["user-book", "move", "--id", "1", "--shelf", "wishlist"]
);
define_cli_auth_test!(
    test_set_book_club_requires_authentication,
    &["user-book", "set-book-club", "--id", "1", "--book-club"]
);
define_cli_auth_test!(
    test_remove_user_book_requires_authentication,
    &["user-book", "remove", "--id", "1"]
);

#[test]
fn test_add_user_book_to_library() {
    let token = create_token("test-add-user-book");
    let author_id = create_author("UB Add Author", &token);
    let book_id = create_book("UB Add Book", &author_id, &token);

    let output = run_booklog(
        &["user-book", "add", "--book-id", &book_id],
        &[("BOOKLOG_TOKEN", &token)],
    );

    assert!(
        output.status.success(),
        "user-book add should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let user_book: Value = serde_json::from_str(&stdout).expect("Should output valid JSON");
    assert!(user_book["id"].is_i64(), "Should have an ID");
    assert_eq!(user_book["shelf"], "library");
}

#[test]
fn test_add_user_book_to_wishlist() {
    let token = create_token("test-add-user-book-wish");
    let author_id = create_author("UB Wishlist Author", &token);
    let book_id = create_book("UB Wishlist Book", &author_id, &token);

    let output = run_booklog(
        &[
            "user-book",
            "add",
            "--book-id",
            &book_id,
            "--shelf",
            "wishlist",
        ],
        &[("BOOKLOG_TOKEN", &token)],
    );

    assert!(output.status.success());

    let user_book: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(user_book["shelf"], "wishlist");
}

#[test]
fn test_list_user_books_shows_added_entry() {
    let token = create_token("test-list-user-books");
    let author_id = create_author("UB List Author", &token);
    let book_id = create_book("UB List Book", &author_id, &token);

    // Add a book to the library
    let add_output = run_booklog(
        &["user-book", "add", "--book-id", &book_id],
        &[("BOOKLOG_TOKEN", &token)],
    );
    assert!(add_output.status.success());

    let add_result: Value = serde_json::from_slice(&add_output.stdout).unwrap();
    let user_book_id = add_result["id"].as_i64().unwrap();

    // List and verify the entry appears
    let list_output = run_booklog(&["user-book", "list"], &[("BOOKLOG_TOKEN", &token)]);
    assert!(list_output.status.success());

    let items: Value = serde_json::from_slice(&list_output.stdout).unwrap();
    assert!(items.is_array());

    let found = items
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"].as_i64() == Some(user_book_id));
    assert!(found, "Should find the added user book in the list");
}

#[test]
fn test_move_user_book_between_shelves() {
    let token = create_token("test-move-user-book");
    let author_id = create_author("UB Move Author", &token);
    let book_id = create_book("UB Move Book", &author_id, &token);

    // Add to library
    let add_output = run_booklog(
        &["user-book", "add", "--book-id", &book_id],
        &[("BOOKLOG_TOKEN", &token)],
    );
    assert!(add_output.status.success());

    let add_result: Value = serde_json::from_slice(&add_output.stdout).unwrap();
    let user_book_id = add_result["id"].as_i64().unwrap().to_string();

    // Move to wishlist
    let move_output = run_booklog(
        &[
            "user-book",
            "move",
            "--id",
            &user_book_id,
            "--shelf",
            "wishlist",
        ],
        &[("BOOKLOG_TOKEN", &token)],
    );

    assert!(
        move_output.status.success(),
        "move should succeed: {}",
        String::from_utf8_lossy(&move_output.stderr)
    );

    let moved: Value = serde_json::from_slice(&move_output.stdout).unwrap();
    assert_eq!(moved["shelf"], "wishlist");
}

#[test]
fn test_set_book_club_flag() {
    let token = create_token("test-set-book-club");
    let author_id = create_author("UB Club Author", &token);
    let book_id = create_book("UB Club Book", &author_id, &token);

    let add_output = run_booklog(
        &["user-book", "add", "--book-id", &book_id],
        &[("BOOKLOG_TOKEN", &token)],
    );
    assert!(add_output.status.success());

    let add_result: Value = serde_json::from_slice(&add_output.stdout).unwrap();
    let user_book_id = add_result["id"].as_i64().unwrap().to_string();
    assert_eq!(add_result["book_club"], false);

    // Set book club to true
    let output = run_booklog(
        &[
            "user-book",
            "set-book-club",
            "--id",
            &user_book_id,
            "--book-club",
        ],
        &[("BOOKLOG_TOKEN", &token)],
    );

    assert!(
        output.status.success(),
        "set-book-club should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let updated: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(updated["book_club"], true);
}

#[test]
fn test_remove_user_book() {
    let token = create_token("test-remove-user-book");
    let author_id = create_author("UB Remove Author", &token);
    let book_id = create_book("UB Remove Book", &author_id, &token);

    let add_output = run_booklog(
        &["user-book", "add", "--book-id", &book_id],
        &[("BOOKLOG_TOKEN", &token)],
    );
    assert!(add_output.status.success());

    let add_result: Value = serde_json::from_slice(&add_output.stdout).unwrap();
    let user_book_id = add_result["id"].as_i64().unwrap().to_string();

    // Remove
    let output = run_booklog(
        &["user-book", "remove", "--id", &user_book_id],
        &[("BOOKLOG_TOKEN", &token)],
    );

    assert!(
        output.status.success(),
        "remove should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify it's gone from the list
    let list_output = run_booklog(&["user-book", "list"], &[("BOOKLOG_TOKEN", &token)]);
    assert!(list_output.status.success());

    let items: Value = serde_json::from_slice(&list_output.stdout).unwrap();
    let found = items
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"].as_i64().map(|id| id.to_string()) == Some(user_book_id.clone()));
    assert!(!found, "Removed user book should not appear in list");
}
