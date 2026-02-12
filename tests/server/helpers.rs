use std::sync::Arc;

use booklog::application::routes::app_router;
use booklog::application::state::{AppState, AppStateConfig};
use booklog::domain::repositories::{
    AuthorRepository, BookRepository, SessionRepository, TimelineEventRepository, TokenRepository,
    UserRepository,
};
use booklog::domain::users::NewUser;
use reqwest::Client;
use serde::{Serialize, de::DeserializeOwned};
use tokio::net::TcpListener;
use tokio::task::AbortHandle;
use webauthn_rs::prelude::*;

pub struct TestApp {
    pub address: String,
    pub pool: booklog::infrastructure::database::DatabasePool,
    pub author_repo: Arc<dyn AuthorRepository>,
    pub book_repo: Arc<dyn BookRepository>,
    #[allow(dead_code)]
    pub timeline_repo: Arc<dyn TimelineEventRepository>,
    #[allow(dead_code)]
    pub user_repo: Option<Arc<dyn UserRepository>>,
    #[allow(dead_code)]
    pub token_repo: Option<Arc<dyn TokenRepository>>,
    pub session_repo: Option<Arc<dyn SessionRepository>>,
    pub auth_token: Option<String>,
    #[allow(dead_code)]
    pub mock_server: Option<wiremock::MockServer>,
    server_handle: AbortHandle,
}

impl TestApp {
    pub fn api_url(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.address, path)
    }

    pub fn page_url(&self, path: &str) -> String {
        format!("{}{}", self.address, path)
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        self.server_handle.abort();
    }
}

fn test_webauthn() -> Arc<Webauthn> {
    #[allow(clippy::expect_used)]
    let rp_origin = url::Url::parse("http://localhost:0").expect("valid URL");
    #[allow(clippy::expect_used)]
    Arc::new(
        WebauthnBuilder::new("localhost", &rp_origin)
            .expect("valid RP config")
            .rp_name("Booklog Test")
            .build()
            .expect("valid WebAuthn"),
    )
}

pub async fn spawn_app() -> TestApp {
    let database = booklog::infrastructure::database::Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory database");

    spawn_app_inner(database, test_state_config(), None).await
}

fn test_state_config() -> AppStateConfig {
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    AppStateConfig {
        webauthn: test_webauthn(),
        insecure_cookies: true,
        openrouter_url: booklog::infrastructure::ai::OPENROUTER_URL.to_string(),
        openrouter_api_key: String::new(),
        openrouter_model: "openrouter/free".to_string(),
        stats_invalidator: booklog::application::services::StatsInvalidator::new(tx),
    }
}

async fn spawn_app_inner(
    database: booklog::infrastructure::database::Database,
    config: AppStateConfig,
    mock_server: Option<wiremock::MockServer>,
) -> TestApp {
    let pool = database.clone_pool();
    let state = AppState::from_database(&database, config);

    // Clone repos we need for TestApp before consuming state in the router
    let author_repo = state.author_repo.clone();
    let book_repo = state.book_repo.clone();
    let timeline_repo = state.timeline_repo.clone();
    let user_repo = state.user_repo.clone();
    let token_repo = state.token_repo.clone();
    let session_repo = state.session_repo.clone();

    let app = app_router(state);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to random port");

    let local_addr = listener.local_addr().expect("Failed to get local address");
    let address = format!("http://{}", local_addr);

    let server_handle = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .expect("Server failed to start");
    })
    .abort_handle();

    TestApp {
        address,
        pool,
        author_repo,
        book_repo,
        timeline_repo,
        user_repo: Some(user_repo),
        token_repo: Some(token_repo),
        session_repo: Some(session_repo),
        auth_token: None,
        mock_server,
        server_handle,
    }
}

pub async fn spawn_app_with_auth() -> TestApp {
    let app = spawn_app().await;
    add_auth_to_app(app).await
}

async fn add_auth_to_app(mut app: TestApp) -> TestApp {
    // Create user with UUID (no password)
    let user_uuid = uuid::Uuid::new_v4().to_string();
    let admin_user = NewUser::new("admin".to_string(), user_uuid);

    let admin_user = app
        .user_repo
        .as_ref()
        .unwrap()
        .insert(admin_user)
        .await
        .expect("Failed to create admin user");

    // Promote to admin (migration only promotes at migration time when no users exist)
    sqlx::query("UPDATE users SET is_admin = 1 WHERE id = ?")
        .bind(i64::from(admin_user.id))
        .execute(&app.pool)
        .await
        .expect("Failed to promote user to admin");

    // Create a token for testing via direct DB insert
    use booklog::domain::tokens::NewToken;
    use booklog::infrastructure::auth::{generate_token, hash_token};

    let token_value = generate_token().expect("Failed to generate token");
    let token_hash = hash_token(&token_value);
    let token = NewToken::new(admin_user.id, token_hash, "test-token".to_string());

    app.token_repo
        .as_ref()
        .unwrap()
        .insert(token)
        .await
        .expect("Failed to insert token");

    app.auth_token = Some(token_value);
    app
}

/// Generic helper: POST a JSON payload and deserialize the response.
/// Automatically attaches the auth token if the test app has one.
pub async fn create_entity<P: Serialize, R: DeserializeOwned>(
    app: &TestApp,
    path: &str,
    payload: &P,
) -> R {
    let client = Client::new();
    let mut request = client.post(app.api_url(path)).json(payload);

    if let Some(token) = &app.auth_token {
        request = request.bearer_auth(token);
    }

    let response = request
        .send()
        .await
        .unwrap_or_else(|e| panic!("failed to create entity at {path}: {e}"));

    response
        .json()
        .await
        .unwrap_or_else(|e| panic!("failed to deserialize entity from {path}: {e}"))
}

pub async fn create_author_with_payload(
    app: &TestApp,
    payload: booklog::domain::authors::NewAuthor,
) -> booklog::domain::authors::Author {
    create_entity(app, "/authors", &payload).await
}

pub async fn create_default_author(app: &TestApp) -> booklog::domain::authors::Author {
    create_author_with_name(app, "Test Author").await
}

pub async fn create_author_with_name(
    app: &TestApp,
    name: &str,
) -> booklog::domain::authors::Author {
    create_author_with_payload(
        app,
        booklog::domain::authors::NewAuthor {
            name: name.to_string(),
            created_at: None,
        },
    )
    .await
}

pub async fn create_genre_with_payload(
    app: &TestApp,
    payload: booklog::domain::genres::NewGenre,
) -> booklog::domain::genres::Genre {
    create_entity(app, "/genres", &payload).await
}

pub async fn create_default_genre(app: &TestApp) -> booklog::domain::genres::Genre {
    create_genre_with_name(app, "Test Genre").await
}

pub async fn create_genre_with_name(app: &TestApp, name: &str) -> booklog::domain::genres::Genre {
    create_genre_with_payload(
        app,
        booklog::domain::genres::NewGenre {
            name: name.to_string(),
            created_at: None,
        },
    )
    .await
}

pub async fn create_default_book(
    app: &TestApp,
    author_id: booklog::domain::ids::AuthorId,
) -> booklog::domain::book_items::Book {
    create_entity(
        app,
        "/books",
        &booklog::domain::book_items::NewBook {
            title: "Test Book".to_string(),
            authors: vec![booklog::domain::book_items::BookAuthor {
                author_id,
                role: booklog::domain::book_items::AuthorRole::default(),
            }],
            isbn: None,
            description: None,
            page_count: None,
            year_published: None,
            publisher: None,
            language: None,
            primary_genre_id: None,
            secondary_genre_id: None,
            created_at: None,
        },
    )
    .await
}

pub async fn create_default_reading(
    app: &TestApp,
    book_id: booklog::domain::ids::BookId,
) -> booklog::domain::readings::Reading {
    create_entity(
        app,
        "/readings",
        &booklog::domain::readings::NewReading {
            user_id: booklog::domain::ids::UserId::new(1),
            book_id,
            status: booklog::domain::readings::ReadingStatus::Reading,
            format: Some(booklog::domain::readings::ReadingFormat::Physical),
            started_at: None,
            finished_at: None,
            rating: None,
            quick_reviews: Vec::new(),
            created_at: None,
        },
    )
    .await
}

/// Asserts that the response has valid Datastar fragment headers
pub fn assert_datastar_headers(response: &reqwest::Response, expected_selector: &str) {
    assert_datastar_headers_with_mode(response, expected_selector, "replace");
}

pub fn assert_datastar_headers_with_mode(
    response: &reqwest::Response,
    expected_selector: &str,
    expected_mode: &str,
) {
    let selector = response
        .headers()
        .get("datastar-selector")
        .and_then(|v| v.to_str().ok());
    assert_eq!(
        selector,
        Some(expected_selector),
        "Expected datastar-selector header to be '{}', got {:?}",
        expected_selector,
        selector
    );

    let mode = response
        .headers()
        .get("datastar-mode")
        .and_then(|v| v.to_str().ok());
    assert_eq!(
        mode,
        Some(expected_mode),
        "Expected datastar-mode header to be '{}', got {:?}",
        expected_mode,
        mode
    );
}

/// Asserts that the response body is an HTML fragment (not a full page)
pub fn assert_html_fragment(body: &str) {
    assert!(
        !body.contains("<!DOCTYPE"),
        "Expected HTML fragment, but found DOCTYPE declaration"
    );
    assert!(
        !body.contains("<html"),
        "Expected HTML fragment, but found <html> tag"
    );
}

/// Asserts that the body contains full HTML page structure
pub fn assert_full_page(body: &str) {
    assert!(
        body.contains("<!DOCTYPE") || body.contains("<html"),
        "Expected full HTML page with DOCTYPE or <html> tag"
    );
}

/// Creates a session for the authenticated user and returns the raw session token
/// to use as a `booklog_session` cookie value.
pub async fn create_session(app: &TestApp) -> String {
    use booklog::domain::sessions::NewSession;
    use booklog::infrastructure::auth::{generate_session_token, hash_token};

    let session_token = generate_session_token();
    let session_hash = hash_token(&session_token);

    // Get the user ID from the auth token
    let token_hash = hash_token(app.auth_token.as_ref().expect("auth token required"));
    let token = app
        .token_repo
        .as_ref()
        .expect("token_repo required")
        .get_by_token_hash(&token_hash)
        .await
        .expect("failed to find token");

    let now = chrono::Utc::now();
    #[allow(clippy::expect_used)]
    let expires_at = now
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("timestamp overflow");

    let new_session = NewSession::new(token.user_id, session_hash, now, expires_at);

    app.session_repo
        .as_ref()
        .expect("session_repo required")
        .insert(new_session)
        .await
        .expect("failed to create session");

    session_token
}

/// POST a form-encoded payload with session cookie auth.
/// Uses `redirect::Policy::none()` so tests can assert the 303 redirect itself.
pub async fn post_form(
    app: &TestApp,
    path: &str,
    form_body: &[(impl AsRef<str> + Serialize, impl AsRef<str> + Serialize)],
) -> reqwest::Response {
    let session_token = create_session(app).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    client
        .post(app.api_url(path))
        .header("Cookie", format!("booklog_session={session_token}"))
        .form(form_body)
        .send()
        .await
        .expect("failed to POST form")
}

/// POST a form-encoded payload with session cookie auth and Datastar headers.
pub async fn post_form_datastar(
    app: &TestApp,
    path: &str,
    form_body: &[(impl AsRef<str> + Serialize, impl AsRef<str> + Serialize)],
) -> reqwest::Response {
    let session_token = create_session(app).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    client
        .post(app.api_url(path))
        .header("Cookie", format!("booklog_session={session_token}"))
        .header("datastar-request", "true")
        .form(form_body)
        .send()
        .await
        .expect("failed to POST form with datastar")
}

/// PUT a form-encoded payload with session cookie auth.
/// Uses `redirect::Policy::none()` so tests can assert the 303 redirect itself.
pub async fn put_form(
    app: &TestApp,
    path: &str,
    form_body: &[(impl AsRef<str> + Serialize, impl AsRef<str> + Serialize)],
) -> reqwest::Response {
    let session_token = create_session(app).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    client
        .put(app.api_url(path))
        .header("Cookie", format!("booklog_session={session_token}"))
        .form(form_body)
        .send()
        .await
        .expect("failed to PUT form")
}

/// PUT a form-encoded payload with session cookie auth and Datastar headers.
pub async fn put_form_datastar(
    app: &TestApp,
    path: &str,
    form_body: &[(impl AsRef<str> + Serialize, impl AsRef<str> + Serialize)],
) -> reqwest::Response {
    let session_token = create_session(app).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    client
        .put(app.api_url(path))
        .header("Cookie", format!("booklog_session={session_token}"))
        .header("datastar-request", "true")
        .form(form_body)
        .send()
        .await
        .expect("failed to PUT form with datastar")
}

/// Create a second (non-admin) user and return its bearer token.
pub async fn create_non_admin_token(app: &TestApp) -> String {
    use booklog::domain::tokens::NewToken;
    use booklog::infrastructure::auth::{generate_token, hash_token};

    let user_uuid = uuid::Uuid::new_v4().to_string();
    let non_admin_user = NewUser::new("non-admin".to_string(), user_uuid);

    let non_admin_user = app
        .user_repo
        .as_ref()
        .unwrap()
        .insert(non_admin_user)
        .await
        .expect("Failed to create non-admin user");

    let token_value = generate_token().expect("Failed to generate token");
    let token_hash = hash_token(&token_value);
    let token = NewToken::new(non_admin_user.id, token_hash, "non-admin-token".to_string());

    app.token_repo
        .as_ref()
        .unwrap()
        .insert(token)
        .await
        .expect("Failed to insert non-admin token");

    token_value
}

pub async fn spawn_app_with_openrouter_mock() -> TestApp {
    let mock_server = wiremock::MockServer::start().await;
    let openrouter_url = format!("{}/api/v1/chat/completions", mock_server.uri());

    let database = booklog::infrastructure::database::Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory database");

    let app = spawn_app_inner(
        database,
        AppStateConfig {
            openrouter_url,
            ..test_state_config()
        },
        Some(mock_server),
    )
    .await;

    add_auth_to_app(app).await
}
