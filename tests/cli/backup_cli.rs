use super::helpers::{create_token, run_booklog};
use crate::test_macros::define_cli_auth_test;

define_cli_auth_test!(backup_requires_auth, &["backup"]);

#[test]
fn backup_produces_valid_json() {
    let token = create_token("backup-test");

    let output = run_booklog(&["backup"], &[("BOOKLOG_TOKEN", &token)]);

    assert!(
        output.status.success(),
        "backup command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let data: serde_json::Value =
        serde_json::from_str(&stdout).expect("backup output is not valid JSON");

    assert_eq!(data["version"], 3);
    assert!(data["authors"].is_array());
    assert!(data["books"].is_array());
    assert!(data["book_authors"].is_array());
    assert!(data["readings"].is_array());
    assert!(data["timeline_events"].is_array());
}
