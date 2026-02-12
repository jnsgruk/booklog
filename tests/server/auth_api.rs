use reqwest::{Client, StatusCode};
use serde_json::json;

use crate::helpers::{create_non_admin_token, spawn_app, spawn_app_with_auth};

// --- WebAuthn endpoint tests ---

#[tokio::test]
async fn test_register_start_requires_valid_token() {
    let app = spawn_app().await;
    let client = Client::new();

    let response = client
        .post(&format!("{}/api/v1/webauthn/register/start", app.address))
        .json(&json!({
            "token": "invalid-registration-token",
            "display_name": "Test User"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "register/start with invalid token should return 401"
    );
}

#[tokio::test]
async fn test_register_finish_requires_valid_challenge() {
    let app = spawn_app().await;
    let client = Client::new();

    let response = client
        .post(&format!("{}/api/v1/webauthn/register/finish", app.address))
        .json(&json!({
            "challenge_id": "nonexistent-challenge",
            "passkey_name": "my-passkey",
            "credential": {}
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert!(
        response.status().is_client_error(),
        "register/finish with invalid challenge should fail, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_auth_start_returns_404_with_no_users() {
    let app = spawn_app().await;
    let client = Client::new();

    let response = client
        .get(&format!("{}/api/v1/webauthn/auth/start", app.address))
        .send()
        .await
        .expect("Failed to send request");

    // When no users/passkeys exist, auth/start should return 404
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "auth/start with no users should return 404"
    );
}

#[tokio::test]
async fn test_auth_finish_requires_valid_challenge() {
    let app = spawn_app().await;
    let client = Client::new();

    let response = client
        .post(&format!("{}/api/v1/webauthn/auth/finish", app.address))
        .json(&json!({
            "challenge_id": "nonexistent-challenge",
            "credential": {}
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert!(
        response.status().is_client_error(),
        "auth/finish with invalid challenge should fail, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_passkey_add_requires_authentication() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .post(&format!("{}/api/v1/webauthn/passkey/start", app.address))
        .json(&json!({}))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "passkey/start should require authentication"
    );
}

// --- Auth edge cases ---

#[tokio::test]
async fn test_expired_session_is_rejected() {
    use booklog::domain::sessions::NewSession;
    use booklog::infrastructure::auth::{generate_session_token, hash_token};

    let app = spawn_app_with_auth().await;

    // Get user_id from existing token
    let token_hash = hash_token(app.auth_token.as_ref().unwrap());
    let token = app
        .token_repo
        .as_ref()
        .unwrap()
        .get_by_token_hash(&token_hash)
        .await
        .unwrap();

    // Create a session that already expired
    let session_token = generate_session_token();
    let session_hash = hash_token(&session_token);
    let now = chrono::Utc::now();
    let expired = now - chrono::Duration::hours(1);
    let new_session = NewSession::new(token.user_id, session_hash, expired, expired);

    app.session_repo
        .as_ref()
        .unwrap()
        .insert(new_session)
        .await
        .unwrap();

    // Use expired session — should fail
    let client = Client::new();
    let response = client
        .post(&app.api_url("/authors"))
        .header("Cookie", format!("booklog_session={session_token}"))
        .json(&json!({"name": "Should Fail"}))
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Expired session should be rejected"
    );
}

#[tokio::test]
async fn test_malformed_bearer_token_is_rejected() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    // Empty bearer
    let response = client
        .post(&app.api_url("/authors"))
        .header("Authorization", "Bearer ")
        .json(&json!({"name": "Should Fail"}))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Not bearer scheme
    let response = client
        .post(&app.api_url("/authors"))
        .header("Authorization", "Basic dXNlcjpwYXNz")
        .json(&json!({"name": "Should Fail"}))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_non_admin_user_can_create_entities() {
    let app = spawn_app_with_auth().await;
    let non_admin_token = create_non_admin_token(&app).await;
    let client = Client::new();

    // Non-admin should be able to create authors (not admin-only)
    let response = client
        .post(&app.api_url("/authors"))
        .bearer_auth(&non_admin_token)
        .json(&json!({"name": "Non-Admin Author"}))
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "Non-admin should be able to create entities"
    );
}

#[tokio::test]
async fn test_create_token_requires_authentication() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    // Token creation now requires session or bearer token auth
    let response = client
        .post(&app.api_url("/tokens"))
        .json(&json!({ "name": "test-token" }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_token_with_bearer_auth() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();
    let auth_token = app.auth_token.as_ref().unwrap();

    // Create a token using bearer auth
    let response = client
        .post(&app.api_url("/tokens"))
        .bearer_auth(auth_token)
        .json(&json!({ "name": "new-token" }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(body.get("id").is_some());
    assert_eq!(body.get("name").unwrap(), "new-token");
    assert!(body.get("token").is_some());

    // Token should be a non-empty string
    let token = body.get("token").unwrap().as_str().unwrap();
    assert!(!token.is_empty());
}

#[tokio::test]
async fn test_list_tokens_requires_authentication() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    // Try to list tokens without authentication
    let response = client
        .get(&app.api_url("/tokens"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_list_tokens_with_authentication() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();
    let auth_token = app.auth_token.as_ref().unwrap();

    // List tokens with authentication
    let response = client
        .get(&app.api_url("/tokens"))
        .bearer_auth(auth_token)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let tokens: Vec<serde_json::Value> = response.json().await.expect("Failed to parse response");
    // We expect at least 1 token (the one created by spawn_app_with_auth)
    assert!(
        !tokens.is_empty(),
        "Should have at least one token from test setup"
    );
    let test_token = tokens
        .iter()
        .find(|t| t.get("name").unwrap() == "test-token")
        .expect("Could not find test-token");
    assert_eq!(test_token.get("name").unwrap(), "test-token");
}

#[tokio::test]
async fn test_revoke_token() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();
    let auth_token = app.auth_token.as_ref().unwrap();

    // Create a new token to revoke
    let create_response = client
        .post(&app.api_url("/tokens"))
        .bearer_auth(auth_token)
        .json(&json!({ "name": "token-to-revoke" }))
        .send()
        .await
        .expect("Failed to send request");

    let create_body: serde_json::Value = create_response
        .json()
        .await
        .expect("Failed to parse response");
    let token_id = create_body.get("id").unwrap().as_i64().unwrap();

    // Revoke the token
    let response = client
        .post(&app.api_url(&format!("/tokens/{}/revoke", token_id)))
        .bearer_auth(auth_token)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(body.get("revoked_at").is_some());
}

#[tokio::test]
async fn test_revoked_token_cannot_be_used() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();
    let auth_token = app.auth_token.as_ref().unwrap();

    // Create a new token
    let create_response = client
        .post(&app.api_url("/tokens"))
        .bearer_auth(auth_token)
        .json(&json!({ "name": "will-be-revoked" }))
        .send()
        .await
        .expect("Failed to send request");

    let create_body: serde_json::Value = create_response
        .json()
        .await
        .expect("Failed to parse response");
    let new_token = create_body.get("token").unwrap().as_str().unwrap();
    let token_id = create_body.get("id").unwrap().as_i64().unwrap();

    // Revoke it using the original auth token
    client
        .post(&app.api_url(&format!("/tokens/{}/revoke", token_id)))
        .bearer_auth(auth_token)
        .send()
        .await
        .expect("Failed to send request");

    // Try to use the revoked token
    let response = client
        .get(&app.api_url("/tokens"))
        .header("Authorization", format!("Bearer {}", new_token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_endpoints_require_authentication() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    // Try to create an author without authentication
    let response = client
        .post(&app.api_url("/authors"))
        .json(&json!({
            "name": "Test Author"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_endpoints_work_with_authentication() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();
    let auth_token = app.auth_token.as_ref().unwrap();

    // Create an author with authentication
    let response = client
        .post(&app.api_url("/authors"))
        .bearer_auth(auth_token)
        .json(&json!({
            "name": "Test Author"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_invalid_session_cookie_fails() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    // Try to create an author with no session (unauthenticated)
    let response = client
        .post(&app.api_url("/authors"))
        .json(&json!({
            "name": "Invalid Session Author"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Request without valid session should fail"
    );
}

#[tokio::test]
async fn test_fake_session_cookie_fails() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    // Try to use a fake/forged session cookie
    let response = client
        .post(&app.api_url("/authors"))
        .header("Cookie", "booklog_session=fake_session_token_12345")
        .json(&json!({
            "name": "Fake Session Author"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Fake session cookie should not authenticate"
    );
}

#[tokio::test]
async fn test_read_endpoints_dont_require_authentication() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    // List authors without authentication should work
    let response = client
        .get(&app.api_url("/authors"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
}

// --- Backup/Reset admin-only tests ---

#[tokio::test]
async fn test_backup_export_requires_admin() {
    let app = spawn_app_with_auth().await;
    let non_admin_token = create_non_admin_token(&app).await;
    let client = Client::new();

    let response = client
        .get(&app.api_url("/backup"))
        .bearer_auth(&non_admin_token)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_backup_export_works_for_admin() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .get(&app.api_url("/backup"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_backup_restore_requires_admin() {
    let app = spawn_app_with_auth().await;
    let non_admin_token = create_non_admin_token(&app).await;
    let client = Client::new();

    let response = client
        .post(&app.api_url("/backup/restore"))
        .bearer_auth(&non_admin_token)
        .json(&json!({
            "version": 1,
            "created_at": "2024-01-01T00:00:00Z",
            "authors": [],
            "genres": [],
            "books": [],
            "book_authors": [],
            "readings": [],
            "timeline_events": []
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_backup_reset_requires_admin() {
    let app = spawn_app_with_auth().await;
    let non_admin_token = create_non_admin_token(&app).await;
    let client = Client::new();

    let response = client
        .post(&app.api_url("/backup/reset"))
        .bearer_auth(&non_admin_token)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_backup_export_requires_auth() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .get(&app.api_url("/backup"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// --- Scan endpoint auth ---

#[tokio::test]
async fn test_submit_scan_requires_auth() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .post(&app.api_url("/scan"))
        .json(&json!({"author_name": "Author", "book_title": "Book"}))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// --- Stats endpoint auth ---

#[tokio::test]
async fn test_stats_recompute_requires_auth() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .post(&app.api_url("/stats/recompute"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// --- Admin endpoint auth ---

#[tokio::test]
async fn test_admin_invite_requires_admin() {
    let app = spawn_app_with_auth().await;
    let non_admin_token = create_non_admin_token(&app).await;
    let client = Client::new();

    let response = client
        .post(&app.api_url("/admin/invite"))
        .bearer_auth(&non_admin_token)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_invite_requires_auth() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .post(&app.api_url("/admin/invite"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_impersonate_requires_admin() {
    let app = spawn_app_with_auth().await;
    let non_admin_token = create_non_admin_token(&app).await;
    let client = Client::new();

    let response = client
        .post(&app.api_url("/admin/impersonate/999"))
        .bearer_auth(&non_admin_token)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_passkeys_requires_auth() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .get(&app.api_url("/passkeys"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_list_passkeys_with_auth() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .get(&app.api_url("/passkeys"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let passkeys: Vec<serde_json::Value> = response.json().await.expect("Failed to parse");
    // No passkeys initially (only token-based auth in tests)
    assert!(passkeys.is_empty());
}

#[tokio::test]
async fn test_delete_passkey_requires_auth() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .delete(&app.api_url("/passkeys/999"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_delete_nonexistent_passkey_returns_404() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .delete(&app.api_url("/passkeys/999"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_admin_impersonate_requires_auth() {
    let app = spawn_app_with_auth().await;
    let client = Client::new();

    let response = client
        .post(&app.api_url("/admin/impersonate/999"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// --- Rate limiting ---

#[tokio::test]
async fn test_webauthn_rate_limit_returns_429() {
    let app = spawn_app().await;
    let client = Client::new();

    // The auth/start endpoint returns 404 when no users exist, but the rate
    // limiter runs first. Send 10 requests (the configured limit), then verify
    // the 11th is rejected with 429.
    for i in 0..10 {
        let response = client
            .get(&format!("{}/api/v1/webauthn/auth/start", app.address))
            .send()
            .await
            .unwrap_or_else(|_| panic!("request {i} failed"));
        assert_ne!(
            response.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "request {i} should not be rate limited"
        );
    }

    let response = client
        .get(&format!("{}/api/v1/webauthn/auth/start", app.address))
        .send()
        .await
        .expect("rate-limited request failed");

    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "11th request should be rate limited"
    );
}
