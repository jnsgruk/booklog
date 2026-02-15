use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::{info, warn};

use crate::application::auth::AuthenticatedUser;
use crate::application::errors::{ApiError, AppError};
use crate::application::routes::api::macros::define_enriched_get_handler;
use crate::application::routes::support::{
    FlexiblePayload, ListQuery, PayloadSource, empty_string_as_none, impl_has_changes,
    is_datastar_request, validate_update,
};
use crate::application::state::AppState;
use crate::domain::ids::{BookId, ReadingId, UserId};
use crate::domain::listing::ListRequest;
use crate::domain::readings::{
    NewReading, QuickReview, ReadingFilter, ReadingFormat, ReadingSortKey, ReadingStatus,
    ReadingWithBook, UpdateReading,
};
use crate::domain::user_books::{NewUserBook, Shelf, user_book_timeline_event};
use crate::presentation::web::templates::ReadingListTemplate;
use crate::presentation::web::views::{ListNavigator, Paginated, ReadingView};

const READING_PAGE_PATH: &str = "/data?type=readings";
const READING_FRAGMENT_PATH: &str = "/data?type=readings#reading-list";

/// Parse an optional date string in `YYYY-MM-DD` format.
fn parse_optional_date(value: Option<String>) -> Option<chrono::NaiveDate> {
    value.and_then(|s| {
        if s.is_empty() {
            None
        } else {
            chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()
        }
    })
}

/// Parse an optional enum from a string, returning `None` for empty strings.
fn parse_optional_enum<T: std::str::FromStr>(value: Option<String>) -> Option<T> {
    value.and_then(|s| {
        if s.is_empty() {
            None
        } else {
            s.parse::<T>().ok()
        }
    })
}

fn deserialize_quick_reviews<'de, D>(deserializer: D) -> Result<Vec<QuickReview>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Deserialize;
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(Vec::new()),
        Some(serde_json::Value::String(s)) if s.is_empty() => Ok(Vec::new()),
        Some(serde_json::Value::String(s)) => Ok(s
            .split(',')
            .filter_map(|v| QuickReview::from_str_value(v.trim()))
            .collect()),
        Some(serde_json::Value::Array(arr)) => Ok(arr
            .iter()
            .filter_map(|v| v.as_str().and_then(QuickReview::from_str_value))
            .collect()),
        Some(_) => Err(serde::de::Error::custom("invalid quick_reviews")),
    }
}

pub(crate) struct ReadingPageData {
    pub(crate) readings: Paginated<ReadingView>,
    pub(crate) navigator: ListNavigator<ReadingSortKey>,
}

#[tracing::instrument(skip(state))]
pub(crate) async fn load_reading_page(
    state: &AppState,
    filter: ReadingFilter,
    request: ListRequest<ReadingSortKey>,
    search: Option<&str>,
) -> Result<ReadingPageData, AppError> {
    let page = state
        .reading_repo
        .list(filter, &request, search)
        .await
        .map_err(AppError::from)?;

    let (readings, navigator) = crate::application::routes::support::build_page_view(
        page,
        request,
        ReadingView::from_domain,
        READING_PAGE_PATH,
        READING_FRAGMENT_PATH,
        search.map(String::from),
    );

    Ok(ReadingPageData {
        readings,
        navigator,
    })
}

#[derive(Debug, Deserialize)]
pub(crate) struct NewReadingSubmission {
    book_id: BookId,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    started_at: Option<String>,
    #[serde(default)]
    finished_at: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    rating: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_quick_reviews")]
    quick_reviews: Vec<QuickReview>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    book_club: Option<bool>,
}

impl NewReadingSubmission {
    fn is_on_shelf(&self) -> bool {
        self.status.as_deref() == Some("on_shelf")
    }

    fn into_new_reading(self, user_id: crate::domain::ids::UserId) -> Result<NewReading, AppError> {
        let book_id = self.book_id;
        if book_id.into_inner() <= 0 {
            return Err(AppError::validation("invalid book id"));
        }

        let status = self
            .status
            .and_then(|s| s.parse::<ReadingStatus>().ok())
            .unwrap_or_default();

        let format = parse_optional_enum::<ReadingFormat>(self.format);

        let started_at = parse_optional_date(self.started_at);
        let finished_at = parse_optional_date(self.finished_at);

        let rating = self.rating.filter(|&r| r != 0.0);
        if let Some(rating) = rating
            && !crate::domain::formatting::is_valid_rating(rating)
        {
            return Err(AppError::validation(
                "rating must be between 0.5 and 5, in 0.5 increments",
            ));
        }

        Ok(NewReading {
            user_id,
            book_id,
            status,
            format,
            started_at,
            finished_at,
            rating,
            quick_reviews: self.quick_reviews,
            created_at: self.created_at,
        })
    }
}

#[tracing::instrument(skip(state, auth_user, headers, query))]
pub(crate) async fn create_reading(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
    payload: FlexiblePayload<NewReadingSubmission>,
) -> Result<Response, ApiError> {
    let (request, search) = query.into_request_and_search::<ReadingSortKey>();
    let (submission, source) = payload.into_parts();
    let user_id = auth_user.effective.id;
    let book_club = submission.book_club;
    let book_id = submission.book_id;

    // "On Shelf" creates a UserBook without a Reading
    if submission.is_on_shelf() {
        return create_on_shelf(&state, user_id, book_id, book_club, &headers, source)
            .await
            .map_err(ApiError::from);
    }

    let new_reading = submission
        .into_new_reading(user_id)
        .map_err(ApiError::from)?;

    let reading = state
        .reading_service
        .create(new_reading)
        .await
        .map_err(AppError::from)?;

    // Set book_club flag on the auto-created UserBook if requested
    if book_club == Some(true)
        && let Ok(user_book) = state
            .user_book_repo
            .get_by_user_and_book(user_id, book_id)
            .await
        && let Err(err) = state.user_book_repo.set_book_club(user_book.id, true).await
    {
        tracing::warn!(error = %err, "failed to set book_club on user_book");
    }

    info!(reading_id = %reading.id, "reading created");
    state.stats_invalidator.invalidate(user_id);

    let detail_url = format!("/readings/{}", reading.id);

    if is_datastar_request(&headers) {
        let from_reading_page = headers
            .get("referer")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|r| r.contains("type=readings"));

        if from_reading_page {
            render_reading_list_fragment(state, request, search, true)
                .await
                .map_err(ApiError::from)
        } else {
            crate::application::routes::support::render_redirect_script(&detail_url)
                .map_err(ApiError::from)
        }
    } else if matches!(source, PayloadSource::Form) {
        Ok(Redirect::to(&detail_url).into_response())
    } else {
        let enriched = state
            .reading_repo
            .get_with_book(reading.id)
            .await
            .map_err(AppError::from)?;
        Ok((StatusCode::CREATED, Json(enriched)).into_response())
    }
}

#[tracing::instrument(skip(state))]
pub(crate) async fn list_readings(
    State(state): State<AppState>,
    Query(params): Query<ReadingsQuery>,
) -> Result<Json<Vec<ReadingWithBook>>, ApiError> {
    let filter = match params.book_id {
        Some(book_id) => ReadingFilter::for_book(book_id),
        None => ReadingFilter::all(),
    };
    let request = ListRequest::show_all(
        ReadingSortKey::CreatedAt,
        crate::domain::listing::SortDirection::Desc,
    );
    let page = state
        .reading_repo
        .list(filter, &request, None)
        .await
        .map_err(AppError::from)?;
    Ok(Json(page.items))
}

define_enriched_get_handler!(
    get_reading,
    ReadingId,
    ReadingWithBook,
    reading_repo,
    get_with_book
);

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateReadingSubmission {
    #[serde(default)]
    book_id: Option<BookId>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    started_at: Option<String>,
    #[serde(default)]
    finished_at: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    rating: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_quick_reviews")]
    quick_reviews: Vec<QuickReview>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    book_club: Option<bool>,
}

impl UpdateReadingSubmission {
    fn into_update(self) -> Result<UpdateReading, AppError> {
        let rating = self.rating.filter(|&r| r != 0.0);
        if let Some(rating) = rating
            && !crate::domain::formatting::is_valid_rating(rating)
        {
            return Err(AppError::validation(
                "rating must be between 0.5 and 5, in 0.5 increments",
            ));
        }

        let format = parse_optional_enum::<ReadingFormat>(self.format);

        Ok(UpdateReading {
            book_id: self.book_id,
            status: self.status.and_then(|s| s.parse::<ReadingStatus>().ok()),
            format,
            started_at: parse_optional_date(self.started_at),
            finished_at: parse_optional_date(self.finished_at),
            rating,
            quick_reviews: if self.quick_reviews.is_empty() {
                None
            } else {
                Some(self.quick_reviews)
            },
            created_at: self.created_at,
        })
    }
}

impl_has_changes!(
    UpdateReading,
    book_id,
    status,
    format,
    started_at,
    finished_at,
    rating,
    quick_reviews,
    created_at
);

#[tracing::instrument(skip(state, auth_user, headers, query))]
pub(crate) async fn update_reading(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
    Path(id): Path<ReadingId>,
    Query(query): Query<ListQuery>,
    payload: FlexiblePayload<UpdateReadingSubmission>,
) -> Result<Response, ApiError> {
    let (request, search) = query.into_request_and_search::<ReadingSortKey>();
    let (submission, source) = payload.into_parts();
    let book_club = submission.book_club;
    let update = submission.into_update().map_err(ApiError::from)?;

    validate_update(&update, None::<&String>)?;

    let existing = state.reading_repo.get(id).await.map_err(AppError::from)?;
    if existing.user_id != auth_user.effective.id {
        return Err(AppError::NotFound.into());
    }

    let is_finishing = update
        .status
        .as_ref()
        .is_some_and(|s| matches!(s, ReadingStatus::Read));

    let reading = if is_finishing {
        state
            .reading_service
            .finish(id, update)
            .await
            .map_err(AppError::from)?
    } else {
        state
            .reading_repo
            .update(id, update)
            .await
            .map_err(AppError::from)?
    };

    // Update book_club flag on the UserBook if provided
    if let Some(book_club) = book_club
        && let Ok(user_book) = state
            .user_book_repo
            .get_by_user_and_book(auth_user.effective.id, reading.book_id)
            .await
        && user_book.book_club != book_club
        && let Err(err) = state
            .user_book_repo
            .set_book_club(user_book.id, book_club)
            .await
    {
        tracing::warn!(error = %err, "failed to update book_club on user_book");
    }

    info!(%id, "reading updated");
    state.stats_invalidator.invalidate(auth_user.effective.id);
    state
        .timeline_invalidator
        .invalidate("reading", i64::from(id));

    if is_datastar_request(&headers) {
        let from_reading_page = headers
            .get("referer")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|r| r.contains("type=readings"));

        if from_reading_page {
            render_reading_list_fragment(state, request, search, true)
                .await
                .map_err(ApiError::from)
        } else {
            let detail_url = format!("/readings/{id}");
            crate::application::routes::support::render_redirect_script(&detail_url)
                .map_err(ApiError::from)
        }
    } else if matches!(source, PayloadSource::Form) {
        let detail_url = format!("/readings/{id}");
        Ok(Redirect::to(&detail_url).into_response())
    } else {
        let enriched = state
            .reading_repo
            .get_with_book(reading.id)
            .await
            .map_err(AppError::from)?;
        Ok(Json(enriched).into_response())
    }
}

#[tracing::instrument(skip(state, auth_user, headers, query))]
pub(crate) async fn delete_reading(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
    Path(id): Path<ReadingId>,
    Query(query): Query<ListQuery>,
) -> Result<Response, ApiError> {
    let existing = state.reading_repo.get(id).await.map_err(AppError::from)?;
    if existing.user_id != auth_user.effective.id {
        return Err(AppError::NotFound.into());
    }

    let (request, search) = query.into_request_and_search::<ReadingSortKey>();
    state
        .reading_repo
        .delete(id)
        .await
        .map_err(AppError::from)?;

    if let Err(err) = state
        .timeline_repo
        .delete_by_entity("reading", i64::from(id))
        .await
    {
        tracing::warn!(%id, error = %err, "failed to delete reading timeline events");
    }

    info!(%id, "reading deleted");
    state.stats_invalidator.invalidate(auth_user.effective.id);

    if is_datastar_request(&headers) {
        let from_data_page = headers
            .get("referer")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|r| r.contains("type=readings"));

        if from_data_page {
            render_reading_list_fragment(state, request, search, true)
                .await
                .map_err(ApiError::from)
        } else {
            crate::application::routes::support::render_redirect_script("/data?type=readings")
                .map_err(ApiError::from)
        }
    } else {
        Ok(StatusCode::NO_CONTENT.into_response())
    }
}

#[derive(Debug, Deserialize)]
pub struct ReadingsQuery {
    pub book_id: Option<BookId>,
}

async fn create_on_shelf(
    state: &AppState,
    user_id: UserId,
    book_id: BookId,
    book_club: Option<bool>,
    headers: &HeaderMap,
    source: PayloadSource,
) -> Result<Response, AppError> {
    if book_id.into_inner() <= 0 {
        return Err(AppError::validation("invalid book id"));
    }

    // If already in library, redirect to book page
    if state
        .user_book_repo
        .get_by_user_and_book(user_id, book_id)
        .await
        .is_ok()
    {
        let detail_url = format!("/books/{book_id}");
        if is_datastar_request(headers) {
            return crate::application::routes::support::render_redirect_script(&detail_url);
        }
        return Ok(Redirect::to(&detail_url).into_response());
    }

    let new_user_book = NewUserBook {
        user_id,
        book_id,
        shelf: Shelf::Library,
        book_club: book_club.unwrap_or(false),
    };
    let user_book = state
        .user_book_repo
        .insert(new_user_book)
        .await
        .map_err(AppError::from)?;

    info!(user_book_id = %user_book.id, %book_id, "book shelved");
    state.stats_invalidator.invalidate(user_id);

    record_user_book_timeline_event(state, &user_book).await;

    let detail_url = format!("/books/{book_id}");

    if is_datastar_request(headers) {
        crate::application::routes::support::render_redirect_script(&detail_url)
    } else if matches!(source, PayloadSource::Form) {
        Ok(Redirect::to(&detail_url).into_response())
    } else {
        Ok((StatusCode::CREATED, Json(user_book)).into_response())
    }
}

async fn record_user_book_timeline_event(
    state: &AppState,
    user_book: &crate::domain::user_books::UserBook,
) {
    let book = match state.book_repo.get(user_book.book_id).await {
        Ok(b) => b,
        Err(err) => {
            warn!(error = %err, user_book_id = %user_book.id, "failed to fetch book for shelf timeline event");
            return;
        }
    };

    let mut authors = Vec::new();
    if let Ok(enriched) = state.book_repo.get_with_authors(book.id).await {
        for author_info in &enriched.authors {
            if let Ok(author) = state.author_repo.get(author_info.author_id).await {
                authors.push(author);
            }
        }
    }

    if let Err(err) = state
        .timeline_repo
        .insert(user_book_timeline_event(user_book, &book, &authors))
        .await
    {
        warn!(error = %err, user_book_id = %user_book.id, "failed to record shelf timeline event");
    }
}

async fn render_reading_list_fragment(
    state: AppState,
    request: ListRequest<ReadingSortKey>,
    search: Option<String>,
    is_authenticated: bool,
) -> Result<Response, AppError> {
    let ReadingPageData {
        readings,
        navigator,
    } = load_reading_page(&state, ReadingFilter::all(), request, search.as_deref()).await?;

    let template = ReadingListTemplate {
        is_authenticated,
        readings,
        navigator,
    };

    crate::application::routes::support::render_fragment(template, "#reading-list")
}
