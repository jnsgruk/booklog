use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::application::auth::AuthenticatedUser;
use crate::application::errors::{ApiError, AppError};
use crate::application::routes::api::images::save_deferred_image;
use crate::application::routes::api::macros::{define_delete_handler, define_enriched_get_handler};
use crate::application::routes::support::{
    FlexiblePayload, ListQuery, PayloadSource, empty_string_as_none, empty_strings_as_vec_i64,
    impl_has_changes, is_datastar_request, render_redirect_script, validate_update,
};
use crate::application::state::AppState;
use crate::domain::book_items::{
    AuthorRole, BookAuthor, BookSortKey, BookWithAuthors, NewBook, UpdateBook,
};
use crate::domain::ids::{AuthorId, BookId, GenreId};
use crate::domain::images::ImageData;
use crate::domain::listing::ListRequest;
use crate::presentation::web::templates::BookListTemplate;
use crate::presentation::web::views::{BookView, ListNavigator, Paginated};
use tracing::info;

const BOOK_PAGE_PATH: &str = "/data?type=books";
const BOOK_FRAGMENT_PATH: &str = "/data?type=books#book-list";

#[tracing::instrument(skip(state))]
pub(crate) async fn load_book_page(
    state: &AppState,
    request: ListRequest<BookSortKey>,
    search: Option<&str>,
    user_id: Option<crate::domain::ids::UserId>,
) -> Result<(Paginated<BookView>, ListNavigator<BookSortKey>), AppError> {
    let page = state
        .book_repo
        .list(&request, search)
        .await
        .map_err(AppError::from)?;

    let library_ids = if let Some(uid) = user_id {
        state
            .user_book_repo
            .book_ids_for_user(uid, None)
            .await
            .map_err(AppError::from)?
    } else {
        std::collections::HashSet::new()
    };

    Ok(crate::application::routes::support::build_page_view(
        page,
        request,
        |bwa| BookView::from_domain(bwa, &library_ids),
        BOOK_PAGE_PATH,
        BOOK_FRAGMENT_PATH,
        search.map(String::from),
    ))
}

#[derive(Debug, Deserialize)]
pub(crate) struct NewBookSubmission {
    title: String,
    #[serde(default)]
    isbn: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    page_count: Option<i32>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    year_published: Option<i32>,
    #[serde(default)]
    publisher: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    primary_genre_id: Option<i64>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    secondary_genre_id: Option<i64>,
    #[serde(default)]
    primary_genre_id_name: Option<String>,
    #[serde(default)]
    secondary_genre_id_name: Option<String>,
    /// Flat `author_id` field from form submissions (converted to `authors` vec).
    #[serde(default, deserialize_with = "empty_string_as_none")]
    author_id: Option<i64>,
    #[serde(default)]
    authors: Vec<BookAuthor>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    image: ImageData,
}

impl NewBookSubmission {
    async fn into_parts(self, state: &AppState) -> Result<(NewBook, Option<String>), AppError> {
        let title = self.title.trim().to_string();
        if title.is_empty() {
            return Err(AppError::validation("title is required"));
        }

        let mut authors = self.authors;
        if let Some(id) = self.author_id
            && authors.is_empty()
        {
            authors.push(BookAuthor {
                author_id: AuthorId::new(id),
                role: AuthorRole::default(),
            });
        }

        let primary_genre_id = match self.primary_genre_id {
            Some(id) => Some(GenreId::new(id)),
            None => resolve_genre_name(state, self.primary_genre_id_name.as_deref()).await,
        };
        let secondary_genre_id = match self.secondary_genre_id {
            Some(id) => Some(GenreId::new(id)),
            None => resolve_genre_name(state, self.secondary_genre_id_name.as_deref()).await,
        };

        Ok((
            NewBook {
                title,
                isbn: self.isbn,
                description: self.description,
                page_count: self.page_count,
                year_published: self.year_published,
                publisher: self.publisher,
                language: self.language,
                primary_genre_id,
                secondary_genre_id,
                authors,
                created_at: self.created_at,
            },
            self.image.into_inner(),
        ))
    }
}

#[tracing::instrument(skip(state, auth_user, headers, query))]
pub(crate) async fn create_book(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
    payload: FlexiblePayload<NewBookSubmission>,
) -> Result<Response, ApiError> {
    let (request, search) = query.into_request_and_search::<BookSortKey>();
    let (submission, source) = payload.into_parts();
    let (new_book, image_data_url) = submission
        .into_parts(&state)
        .await
        .map_err(ApiError::from)?;
    let new_book = new_book.normalize();
    let user_id = auth_user.effective.id;

    let book = state
        .book_service
        .create(new_book, user_id)
        .await
        .map_err(AppError::from)?;

    info!(book_id = %book.id, title = %book.title, "book created");
    state.stats_invalidator.invalidate(user_id);

    save_deferred_image(
        &state,
        "book",
        i64::from(book.id),
        image_data_url.as_deref(),
    )
    .await;

    let detail_url = format!("/books/{}", book.id);

    if is_datastar_request(&headers) {
        let from_data_page = headers
            .get("referer")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|r| r.contains("type=books"));

        if from_data_page {
            render_book_list_fragment(state, request, search, true)
                .await
                .map_err(ApiError::from)
        } else {
            render_redirect_script(&detail_url).map_err(ApiError::from)
        }
    } else if matches!(source, PayloadSource::Form) {
        Ok(Redirect::to(&detail_url).into_response())
    } else {
        let enriched = state
            .book_repo
            .get_with_authors(book.id)
            .await
            .map_err(AppError::from)?;
        Ok((StatusCode::CREATED, Json(enriched)).into_response())
    }
}

#[tracing::instrument(skip(state))]
pub(crate) async fn list_books(
    State(state): State<AppState>,
) -> Result<Json<Vec<BookWithAuthors>>, ApiError> {
    let books = state.book_repo.list_all().await.map_err(AppError::from)?;
    Ok(Json(books))
}

define_enriched_get_handler!(
    get_book,
    BookId,
    BookWithAuthors,
    book_repo,
    get_with_authors
);

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateBookSubmission {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    isbn: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    page_count: Option<i32>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    year_published: Option<i32>,
    #[serde(default)]
    publisher: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    primary_genre_id: Option<i64>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    secondary_genre_id: Option<i64>,
    #[serde(default)]
    primary_genre_id_name: Option<String>,
    #[serde(default)]
    secondary_genre_id_name: Option<String>,
    /// Flat `author_ids` field from multi-select form submissions.
    #[serde(default, deserialize_with = "empty_strings_as_vec_i64")]
    author_ids: Vec<i64>,
    #[serde(default)]
    authors: Option<Vec<BookAuthor>>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    image: ImageData,
    #[serde(default)]
    selected_cover_id: Option<String>,
}

impl UpdateBookSubmission {
    async fn into_parts(self, state: &AppState) -> (UpdateBook, Option<String>, Option<String>) {
        let authors = if self.authors.is_some() {
            self.authors
        } else if !self.author_ids.is_empty() {
            Some(
                self.author_ids
                    .into_iter()
                    .map(|id| BookAuthor {
                        author_id: AuthorId::new(id),
                        role: AuthorRole::default(),
                    })
                    .collect(),
            )
        } else {
            None
        };

        let primary_genre_id = match self.primary_genre_id {
            Some(id) => Some(Some(GenreId::new(id))),
            None => resolve_genre_name(state, self.primary_genre_id_name.as_deref())
                .await
                .map(Some),
        };
        let secondary_genre_id = match self.secondary_genre_id {
            Some(id) => Some(Some(GenreId::new(id))),
            None => resolve_genre_name(state, self.secondary_genre_id_name.as_deref())
                .await
                .map(Some),
        };

        let update = UpdateBook {
            title: self.title,
            isbn: self.isbn,
            description: self.description,
            page_count: self.page_count,
            year_published: self.year_published,
            publisher: self.publisher,
            language: self.language,
            primary_genre_id,
            secondary_genre_id,
            authors,
            created_at: self.created_at,
        };
        let selected_cover_id = self.selected_cover_id.filter(|s| !s.is_empty());
        (update, self.image.into_inner(), selected_cover_id)
    }
}

impl_has_changes!(
    UpdateBook,
    title,
    isbn,
    description,
    page_count,
    year_published,
    publisher,
    language,
    primary_genre_id,
    secondary_genre_id,
    authors,
    created_at
);

#[tracing::instrument(skip(state, auth_user, headers))]
pub(crate) async fn update_book(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
    Path(id): Path<BookId>,
    payload: FlexiblePayload<UpdateBookSubmission>,
) -> Result<Response, ApiError> {
    let (submission, source) = payload.into_parts();
    let (update, image_data_url, selected_cover_id) = submission.into_parts(&state).await;
    let update = update.normalize();

    validate_update(&update, image_data_url.as_ref())?;

    state
        .book_repo
        .update(id, update)
        .await
        .map_err(AppError::from)?;

    info!(%id, "book updated");
    state.stats_invalidator.invalidate(auth_user.effective.id);
    state.timeline_invalidator.invalidate("book", i64::from(id));

    if let Some(cover_id) = &selected_cover_id {
        super::scan::promote_cover_suggestion(&state, "book", i64::from(id), cover_id).await;
    } else {
        save_deferred_image(&state, "book", i64::from(id), image_data_url.as_deref()).await;
    }

    let enriched = state
        .book_repo
        .get_with_authors(id)
        .await
        .map_err(AppError::from)?;

    let detail_url = format!("/books/{}", enriched.book.id);

    if is_datastar_request(&headers) {
        render_redirect_script(&detail_url).map_err(ApiError::from)
    } else if matches!(source, PayloadSource::Form) {
        Ok(Redirect::to(&detail_url).into_response())
    } else {
        Ok(Json(enriched).into_response())
    }
}

define_delete_handler!(
    delete_book,
    BookId,
    BookSortKey,
    book_repo,
    render_book_list_fragment,
    "type=books",
    "/data?type=books",
    image_type: "book",
    entity_type: "book"
);

/// Resolve an optional genre name to a `GenreId`, creating the genre if needed.
async fn resolve_genre_name(state: &AppState, name: Option<&str>) -> Option<GenreId> {
    let name = name.filter(|s| !s.trim().is_empty())?;
    super::scan::resolve_or_create_genre(state, name).await
}

async fn render_book_list_fragment(
    state: AppState,
    request: ListRequest<BookSortKey>,
    search: Option<String>,
    is_authenticated: bool,
) -> Result<Response, AppError> {
    let (books, navigator) = load_book_page(&state, request, search.as_deref(), None).await?;
    let template = BookListTemplate {
        is_authenticated,
        books,
        navigator,
    };
    crate::application::routes::support::render_fragment(template, "#book-list")
}
