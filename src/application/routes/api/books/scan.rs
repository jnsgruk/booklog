use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::application::auth::AuthenticatedUser;
use crate::application::errors::{ApiError, AppError};
use crate::application::routes::api::images::{resolve_image_url, save_deferred_image};
use crate::application::routes::support::{
    FlexiblePayload, empty_string_as_none, is_datastar_request,
};
use crate::application::state::AppState;
use crate::domain::books::authors::NewAuthor;
use crate::domain::books::books::{AuthorRole, BookAuthor, NewBook};
use crate::domain::errors::RepositoryError;
use crate::domain::ids::{BookId, GenreId};
use crate::domain::images::EntityImage;
use crate::domain::images::ImageData;
use crate::infrastructure::ai::{self, ExtractionInput, Usage};
use crate::infrastructure::cover_fetch;

const COVER_SIGNALS: [&str; 5] = ["_cover-1", "_cover-2", "_cover-3", "_cover-4", "_cover-5"];

#[tracing::instrument(skip(state, auth_user, headers, payload))]
pub(crate) async fn extract_book_scan(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
    payload: FlexiblePayload<ExtractionInput>,
) -> Result<Response, ApiError> {
    let (input, _) = payload.into_parts();

    let available_genres = load_genre_names(&state).await;
    let (result, usage) = ai::extract_book(
        &state.http_client,
        &state.openrouter_url,
        &state.openrouter_api_key,
        &state.openrouter_model,
        &input,
        &available_genres,
    )
    .await
    .map_err(ApiError::from)?;

    crate::application::routes::support::record_ai_usage(
        state.ai_usage_repo.clone(),
        auth_user.effective.id,
        &state.openrouter_model,
        "extract-book-scan",
        usage,
    );

    if !is_datastar_request(&headers) {
        return Ok(Json(result).into_response());
    }

    let (matched_author_id, matched_book_id) = match_existing_entities(&state, &result).await;
    let (primary_genre_id, secondary_genre_id) = resolve_genre_ids(
        &state,
        result.primary_genre.as_deref(),
        result.secondary_genre.as_deref(),
    )
    .await;

    let suggestion_ids =
        fetch_and_store_cover_suggestions(&state, result.cover_image_urls.as_deref()).await;

    let signals = build_extraction_signals(
        result,
        matched_author_id,
        matched_book_id,
        primary_genre_id,
        secondary_genre_id,
        &suggestion_ids,
    );

    crate::application::routes::support::render_signals_json(&signals).map_err(ApiError::from)
}

fn build_extraction_signals(
    result: ai::ExtractedBook,
    matched_author_id: String,
    matched_book_id: String,
    primary_genre_id: Option<GenreId>,
    secondary_genre_id: Option<GenreId>,
    suggestion_ids: &[String],
) -> Vec<(&'static str, serde_json::Value)> {
    use serde_json::Value;

    let mut signals = vec![
        (
            "_author-name",
            Value::String(result.author_name.unwrap_or_default()),
        ),
        (
            "_book-title",
            Value::String(result.title.unwrap_or_default()),
        ),
        ("_book-isbn", Value::String(result.isbn.unwrap_or_default())),
        (
            "_book-description",
            Value::String(result.description.unwrap_or_default()),
        ),
        (
            "_book-pages",
            Value::String(result.page_count.map(|p| p.to_string()).unwrap_or_default()),
        ),
        (
            "_book-year",
            Value::String(
                result
                    .year_published
                    .map(|y| y.to_string())
                    .unwrap_or_default(),
            ),
        ),
        (
            "_book-publisher",
            Value::String(result.publisher.unwrap_or_default()),
        ),
        (
            "_book-language",
            Value::String(result.language.unwrap_or_default()),
        ),
        (
            "_book-primary-genre",
            Value::String(result.primary_genre.unwrap_or_default()),
        ),
        (
            "_book-secondary-genre",
            Value::String(result.secondary_genre.unwrap_or_default()),
        ),
        (
            "_book-primary-genre-id",
            Value::String(
                primary_genre_id
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
            ),
        ),
        (
            "_book-secondary-genre-id",
            Value::String(
                secondary_genre_id
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
            ),
        ),
        ("_scan-extracted", Value::Bool(true)),
        ("_matched-author-id", Value::String(matched_author_id)),
        ("_matched-book-id", Value::String(matched_book_id)),
    ];

    for (i, signal_name) in COVER_SIGNALS.iter().enumerate() {
        let id = suggestion_ids.get(i).cloned().unwrap_or_default();
        signals.push((signal_name, Value::String(id)));
    }

    signals
}

/// Fetch cover images from the URLs returned by the LLM, store them as temporary
/// suggestions, and return their IDs.
async fn fetch_and_store_cover_suggestions(
    state: &AppState,
    urls: Option<&[String]>,
) -> Vec<String> {
    let urls = urls.unwrap_or_default();
    if urls.is_empty() {
        return Vec::new();
    }

    let fetched = cover_fetch::fetch_cover_images(&state.http_client, urls).await;
    let ids: Vec<String> = fetched.iter().map(|s| s.id.clone()).collect();
    for suggestion in fetched {
        if let Err(err) = state.cover_suggestion_repo.insert(suggestion).await {
            tracing::warn!(error = %err, "failed to store cover suggestion");
        }
    }
    ids
}

/// Check if the extracted author/book already exist by name matching.
/// Returns `(matched_author_id, matched_book_id)` as strings (empty if no match).
async fn match_existing_entities(state: &AppState, result: &ai::ExtractedBook) -> (String, String) {
    let author_name = result.author_name.as_deref().unwrap_or_default().trim();
    if author_name.is_empty() {
        return (String::new(), String::new());
    }

    let Ok(existing_author) = state.author_repo.get_by_name(author_name).await else {
        return (String::new(), String::new());
    };

    let matched_author_id = existing_author.id.into_inner().to_string();

    let book_title = result.title.as_deref().unwrap_or_default().trim();
    if book_title.is_empty() {
        return (matched_author_id, String::new());
    }

    let matched_book_id = match state.book_repo.get_by_title(book_title).await {
        Ok(b) => b.id.into_inner().to_string(),
        Err(_) => String::new(),
    };

    (matched_author_id, matched_book_id)
}

#[derive(Debug, Deserialize)]
pub(crate) struct BookScanSubmission {
    #[serde(default)]
    image: ImageData,
    #[serde(default)]
    prompt: Option<String>,
    // Author fields
    #[serde(default)]
    author_name: String,
    // Book fields
    #[serde(default)]
    book_title: String,
    #[serde(default)]
    book_isbn: Option<String>,
    #[serde(default)]
    book_description: Option<String>,
    #[serde(default)]
    book_pages: Option<String>,
    #[serde(default)]
    book_year: Option<String>,
    #[serde(default)]
    book_publisher: Option<String>,
    #[serde(default)]
    book_language: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    book_primary_genre_id: Option<i64>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    book_secondary_genre_id: Option<i64>,
    #[serde(default)]
    book_primary_genre_name: Option<String>,
    #[serde(default)]
    book_secondary_genre_name: Option<String>,
    // Shelf assignment
    #[serde(default)]
    shelf: Option<String>,
    #[serde(default)]
    book_club: Option<bool>,
    // Match tracking
    #[serde(default)]
    matched_book_id: Option<String>,
    // Scan image for book cover
    #[serde(default)]
    scan_image: ImageData,
    // Selected cover suggestion ID (from AI extraction)
    #[serde(default)]
    selected_cover_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ScanResult {
    redirect: String,
    book_id: i64,
}

/// Populate a `BookScanSubmission` from AI extraction when image/prompt is provided.
async fn extract_into_submission(
    state: &AppState,
    submission: &mut BookScanSubmission,
) -> Result<Option<Usage>, ApiError> {
    let input = ExtractionInput {
        image: submission.image.take(),
        prompt: submission.prompt.take(),
    };
    let available_genres = load_genre_names(state).await;
    let (result, usage) = ai::extract_book(
        &state.http_client,
        &state.openrouter_url,
        &state.openrouter_api_key,
        &state.openrouter_model,
        &input,
        &available_genres,
    )
    .await
    .map_err(ApiError::from)?;

    if let Some(name) = result.author_name {
        submission.author_name = name;
    }
    if let Some(title) = result.title {
        submission.book_title = title;
    }
    if result.isbn.is_some() {
        submission.book_isbn = result.isbn;
    }
    if result.description.is_some() {
        submission.book_description = result.description;
    }
    if let Some(pages) = result.page_count {
        submission.book_pages = Some(pages.to_string());
    }
    if let Some(year) = result.year_published {
        submission.book_year = Some(year.to_string());
    }
    if result.publisher.is_some() {
        submission.book_publisher = result.publisher;
    }
    if result.language.is_some() {
        submission.book_language = result.language;
    }
    // Resolve genre names to IDs (auto-create if needed)
    let (primary_id, secondary_id) = resolve_genre_ids(
        state,
        result.primary_genre.as_deref(),
        result.secondary_genre.as_deref(),
    )
    .await;
    if let Some(id) = primary_id {
        submission.book_primary_genre_id = Some(id.into_inner());
    }
    if let Some(id) = secondary_id {
        submission.book_secondary_genre_id = Some(id.into_inner());
    }

    Ok(usage)
}

#[tracing::instrument(skip(state, auth_user, headers))]
pub(crate) async fn submit_scan(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
    payload: FlexiblePayload<BookScanSubmission>,
) -> Result<Response, ApiError> {
    let user_id = auth_user.effective.id;
    let (mut submission, _) = payload.into_parts();

    // If the book already exists (matched during extraction), skip creation
    if let Some(book_id) = parse_matched_book_id(submission.matched_book_id.as_ref()) {
        let scan_image = submission.scan_image.take();
        let selected_cover_id = submission.selected_cover_id.take();
        return submit_existing_book(
            &state,
            &headers,
            book_id,
            &submission,
            scan_image,
            selected_cover_id,
            user_id,
        )
        .await;
    }

    // Check for raw input (image/prompt triggers extraction first)
    let has_raw_input = submission.image.as_deref().is_some_and(|s| !s.is_empty())
        || submission.prompt.as_deref().is_some_and(|s| !s.is_empty());

    // Preserve scan image: either from the dedicated field (two-step Datastar flow)
    // or from the raw image input (one-step API flow, before extraction consumes it)
    let scan_image = submission
        .scan_image
        .take()
        .or_else(|| submission.image.cloned())
        .filter(|s| !s.is_empty());

    if has_raw_input {
        let usage = extract_into_submission(&state, &mut submission).await?;
        crate::application::routes::support::record_ai_usage(
            state.ai_usage_repo.clone(),
            user_id,
            &state.openrouter_model,
            "extract-book-scan",
            usage,
        );
    }

    submit_new_book(&state, &headers, submission, scan_image, user_id).await
}

/// Create a new book from the scan submission fields.
async fn submit_new_book(
    state: &AppState,
    headers: &HeaderMap,
    mut submission: BookScanSubmission,
    scan_image: Option<String>,
    user_id: crate::domain::ids::UserId,
) -> Result<Response, ApiError> {
    if submission.book_title.trim().is_empty() {
        return Err(AppError::validation("book title is required").into());
    }

    let author = resolve_or_create_author(state, &submission.author_name, user_id).await?;

    // Resolve genre names to IDs if user manually edited the genre text
    resolve_submission_genres(state, &mut submission).await;

    let new_book = NewBook {
        title: submission.book_title.trim().to_string(),
        authors: vec![BookAuthor {
            author_id: author.id,
            role: AuthorRole::default(),
        }],
        isbn: normalize_opt(submission.book_isbn),
        description: normalize_opt(submission.book_description),
        page_count: submission.book_pages.and_then(|s| s.parse().ok()),
        year_published: submission.book_year.and_then(|s| s.parse().ok()),
        publisher: normalize_opt(submission.book_publisher),
        language: normalize_opt(submission.book_language),
        primary_genre_id: submission.book_primary_genre_id.map(GenreId::new),
        secondary_genre_id: submission.book_secondary_genre_id.map(GenreId::new),
        created_at: None,
    };

    let book = state
        .book_service
        .create(new_book, user_id)
        .await
        .map_err(AppError::from)?;

    info!(author_id = %author.id, book_id = %book.id, book_title = %book.title, "scan created book");

    // Save cover image: prefer selected suggestion, fall back to captured photo
    let selected_cover_id = submission
        .selected_cover_id
        .as_deref()
        .filter(|s| !s.is_empty());
    if let Some(cover_id) = selected_cover_id {
        promote_cover_suggestion(state, "book", book.id.into_inner(), cover_id).await;
    } else {
        save_deferred_image(state, "book", book.id.into_inner(), scan_image.as_deref()).await;
    }

    let shelf = parse_shelf(submission.shelf.as_deref());
    add_book_to_shelf(
        state,
        user_id,
        book.id,
        shelf,
        submission.book_club.unwrap_or(false),
    )
    .await;
    state.stats_invalidator.invalidate(user_id);

    let redirect = format!("/books/{}", book.id);
    let book_id = book.id.into_inner();

    if is_datastar_request(headers) {
        use serde_json::Value;
        let signals = vec![
            ("_book-id", Value::String(book_id.to_string())),
            ("_scan-success", Value::String(book.title.clone())),
            ("_author-name", Value::String(author.name.clone())),
        ];
        crate::application::routes::support::render_signals_json(&signals).map_err(ApiError::from)
    } else {
        Ok((StatusCode::CREATED, Json(ScanResult { redirect, book_id })).into_response())
    }
}

/// Fill in genre IDs from genre names when the user manually edited the text fields.
async fn resolve_submission_genres(state: &AppState, submission: &mut BookScanSubmission) {
    if submission.book_primary_genre_id.is_none()
        && let Some(name) = submission
            .book_primary_genre_name
            .as_deref()
            .filter(|s| !s.trim().is_empty())
    {
        submission.book_primary_genre_id = resolve_or_create_genre(state, name)
            .await
            .map(GenreId::into_inner);
    }
    if submission.book_secondary_genre_id.is_none()
        && let Some(name) = submission
            .book_secondary_genre_name
            .as_deref()
            .filter(|s| !s.trim().is_empty())
    {
        submission.book_secondary_genre_id = resolve_or_create_genre(state, name)
            .await
            .map(GenreId::into_inner);
    }
}

/// Find an existing author by name, or create a new one.
async fn resolve_or_create_author(
    state: &AppState,
    name: &str,
    user_id: crate::domain::ids::UserId,
) -> Result<crate::domain::books::authors::Author, ApiError> {
    let new_author = NewAuthor {
        name: name.to_string(),
        created_at: None,
    }
    .normalize();

    if new_author.name.is_empty() {
        return Err(AppError::validation("author name is required").into());
    }

    match state.author_repo.get_by_name(&new_author.name).await {
        Ok(existing) => Ok(existing),
        Err(RepositoryError::NotFound) => state
            .author_service
            .create(new_author, user_id)
            .await
            .map_err(AppError::from)
            .map_err(ApiError::from),
        Err(err) => Err(AppError::from(err).into()),
    }
}

/// Add a book to the user's shelf, logging on failure.
async fn add_book_to_shelf(
    state: &AppState,
    user_id: crate::domain::ids::UserId,
    book_id: BookId,
    shelf: crate::domain::user_books::Shelf,
    book_club: bool,
) {
    let new_user_book = crate::domain::user_books::NewUserBook {
        user_id,
        book_id,
        shelf,
        book_club,
    };
    if let Err(err) = state.user_book_repo.insert(new_user_book).await {
        tracing::warn!(error = %err, "failed to create user_book from scan");
    }
}

fn parse_matched_book_id(value: Option<&String>) -> Option<BookId> {
    value
        .map(String::as_str)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<i64>().ok())
        .map(BookId::from)
}

fn parse_shelf(value: Option<&str>) -> crate::domain::user_books::Shelf {
    match value {
        Some("wishlist") => crate::domain::user_books::Shelf::Wishlist,
        _ => crate::domain::user_books::Shelf::Library,
    }
}

fn normalize_opt(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Load all genre names for the AI prompt.
async fn load_genre_names(state: &AppState) -> Vec<String> {
    use crate::domain::genres::GenreSortKey;
    use crate::domain::listing::SortDirection;
    state
        .genre_repo
        .list_all_sorted(GenreSortKey::Name, SortDirection::Asc)
        .await
        .ok()
        .map(|genres| genres.into_iter().map(|g| g.name).collect())
        .unwrap_or_default()
}

/// Resolve genre name strings to `GenreIds`, auto-creating new genres if needed.
pub(crate) async fn resolve_genre_ids(
    state: &AppState,
    primary: Option<&str>,
    secondary: Option<&str>,
) -> (Option<GenreId>, Option<GenreId>) {
    let primary_id = if let Some(name) = primary {
        resolve_or_create_genre(state, name).await
    } else {
        None
    };
    let secondary_id = if let Some(name) = secondary {
        resolve_or_create_genre(state, name).await
    } else {
        None
    };
    (primary_id, secondary_id)
}

pub(crate) async fn resolve_or_create_genre(state: &AppState, name: &str) -> Option<GenreId> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    match state.genre_repo.get_by_name(trimmed).await {
        Ok(genre) => Some(genre.id),
        Err(RepositoryError::NotFound) => {
            let new_genre = crate::domain::genres::NewGenre {
                name: trimmed.to_string(),
                created_at: None,
            };
            match state.genre_repo.insert(new_genre).await {
                Ok(genre) => {
                    info!(genre_id = %genre.id, name = %genre.name, "auto-created genre from extraction");
                    Some(genre.id)
                }
                Err(err) => {
                    tracing::warn!(error = %err, genre_name = %trimmed, "failed to auto-create genre");
                    None
                }
            }
        }
        Err(err) => {
            tracing::warn!(error = %err, genre_name = %trimmed, "failed to look up genre by name");
            None
        }
    }
}

/// Promote a cover suggestion to permanent entity image storage.
pub(crate) async fn promote_cover_suggestion(
    state: &AppState,
    entity_type: &str,
    entity_id: i64,
    cover_id: &str,
) {
    match state.cover_suggestion_repo.get(cover_id).await {
        Ok(suggestion) => {
            let image = EntityImage {
                entity_type: entity_type.to_string(),
                entity_id,
                content_type: suggestion.content_type,
                image_data: suggestion.image_data,
                thumbnail_data: suggestion.thumbnail_data,
            };
            if let Err(err) = state.image_repo.upsert(image).await {
                tracing::warn!(error = %err, cover_id, "failed to promote cover suggestion");
            } else {
                info!(
                    entity_type,
                    entity_id, cover_id, "promoted cover suggestion to entity image"
                );
                // Clean up the used suggestion
                if let Err(err) = state.cover_suggestion_repo.delete(cover_id).await {
                    tracing::warn!(error = %err, cover_id, "failed to delete used cover suggestion");
                }
            }
        }
        Err(err) => {
            tracing::warn!(error = %err, cover_id, "cover suggestion not found for promotion");
        }
    }
}

/// Handle submission when the book already exists — only create a reading if requested.
async fn submit_existing_book(
    state: &AppState,
    headers: &HeaderMap,
    book_id: BookId,
    submission: &BookScanSubmission,
    scan_image: Option<String>,
    selected_cover_id: Option<String>,
    user_id: crate::domain::ids::UserId,
) -> Result<Response, ApiError> {
    let book_with_authors = state
        .book_repo
        .get_with_authors(book_id)
        .await
        .map_err(AppError::from)?;

    let book = &book_with_authors.book;

    // Save cover image if book doesn't have one yet
    if resolve_image_url(state, "book", book.id.into_inner())
        .await
        .is_none()
    {
        let cover_id = selected_cover_id.as_deref().filter(|s| !s.is_empty());
        if let Some(cover_id) = cover_id {
            promote_cover_suggestion(state, "book", book.id.into_inner(), cover_id).await;
        } else {
            save_deferred_image(state, "book", book.id.into_inner(), scan_image.as_deref()).await;
        }
    }

    let shelf = parse_shelf(submission.shelf.as_deref());
    add_book_to_shelf(
        state,
        user_id,
        book.id,
        shelf,
        submission.book_club.unwrap_or(false),
    )
    .await;
    state.stats_invalidator.invalidate(user_id);

    let redirect = format!("/books/{}", book.id);
    let book_id_raw = book.id.into_inner();

    if is_datastar_request(headers) {
        use serde_json::Value;
        let author_name = book_with_authors
            .authors
            .first()
            .map(|a| a.author_name.clone())
            .unwrap_or_default();
        let signals = vec![
            ("_book-id", Value::String(book_id_raw.to_string())),
            ("_scan-success", Value::String(book.title.clone())),
            ("_author-name", Value::String(author_name)),
        ];
        crate::application::routes::support::render_signals_json(&signals).map_err(ApiError::from)
    } else {
        Ok((
            StatusCode::CREATED,
            Json(ScanResult {
                redirect,
                book_id: book_id_raw,
            }),
        )
            .into_response())
    }
}

// --- Fetch covers for an existing book ---

#[derive(Debug, Serialize)]
struct FetchCoversResult {
    suggestion_ids: Vec<String>,
}

#[tracing::instrument(skip(state, auth_user, headers))]
pub(crate) async fn fetch_covers_for_book(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
    Path(id): Path<BookId>,
) -> Result<Response, ApiError> {
    let book_with_authors = state
        .book_repo
        .get_with_authors(id)
        .await
        .map_err(AppError::from)?;

    let book = &book_with_authors.book;
    let author_name = book_with_authors
        .authors
        .first()
        .map_or("", |a| a.author_name.as_str());

    let (result, usage) = ai::fetch_cover_urls(
        &state.http_client,
        &state.openrouter_url,
        &state.openrouter_api_key,
        &state.openrouter_model,
        &book.title,
        author_name,
        book.isbn.as_deref(),
    )
    .await
    .map_err(ApiError::from)?;

    crate::application::routes::support::record_ai_usage(
        state.ai_usage_repo.clone(),
        auth_user.effective.id,
        &state.openrouter_model,
        "fetch-covers",
        usage,
    );

    let suggestion_ids =
        fetch_and_store_cover_suggestions(&state, result.cover_image_urls.as_deref()).await;

    if is_datastar_request(&headers) {
        let mut signals: Vec<(&str, serde_json::Value)> = Vec::new();
        for (i, signal_name) in COVER_SIGNALS.iter().enumerate() {
            let id = suggestion_ids.get(i).cloned().unwrap_or_default();
            signals.push((signal_name, serde_json::Value::String(id)));
        }
        crate::application::routes::support::render_signals_json(&signals).map_err(ApiError::from)
    } else {
        Ok(Json(FetchCoversResult { suggestion_ids }).into_response())
    }
}

// --- Cover suggestion serving endpoints ---

#[tracing::instrument(skip(state))]
pub(crate) async fn get_cover_suggestion(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let suggestion = state
        .cover_suggestion_repo
        .get(&id)
        .await
        .map_err(AppError::from)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &suggestion.content_type)
        .header(header::CACHE_CONTROL, "private, max-age=86400")
        .body(Body::from(suggestion.image_data))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()))
}

#[tracing::instrument(skip(state))]
pub(crate) async fn get_cover_suggestion_thumbnail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let suggestion = state
        .cover_suggestion_repo
        .get_thumbnail(&id)
        .await
        .map_err(AppError::from)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &suggestion.content_type)
        .header(header::CACHE_CONTROL, "private, max-age=86400")
        .body(Body::from(suggestion.thumbnail_data))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()))
}
