use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};

use crate::application::auth::impersonation_info;
use crate::application::errors::map_app_error;
use crate::application::routes::render_html;
use crate::application::state::AppState;
use crate::domain::books::readings::ReadingFilter;
use crate::domain::listing::{ListRequest, PageSize};
use crate::presentation::web::templates::{
    BookDetailTemplate, BookEditTemplate, StartBookTemplate,
};
use crate::presentation::web::views::{
    AuthorOptionView, BookDetailView, BookLibraryInfo, BookReadingCardView, GenreOptionView,
};

#[tracing::instrument(skip(state, cookies))]
pub(crate) async fn book_detail_page(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    Path(id): Path<crate::domain::ids::BookId>,
) -> Result<Response, StatusCode> {
    let is_authenticated = crate::application::routes::is_authenticated(&state, &cookies).await;
    let (is_impersonating, impersonated_username) = impersonation_info(&state, &cookies).await;

    let enriched = state
        .book_repo
        .get_with_authors(id)
        .await
        .map_err(|e| map_app_error(e.into()))?;

    let image_url =
        crate::application::routes::support::image_url(&*state.image_repo, "book", i64::from(id))
            .await;

    let user_id = crate::application::routes::authenticated_user_id(&state, &cookies).await;

    let (library_info, readings, active_reading_id) = if let Some(uid) = user_id {
        let user_book = state
            .user_book_repo
            .get_by_user_and_book(uid, id)
            .await
            .ok();
        let book_club = user_book.as_ref().is_some_and(|ub| ub.book_club);

        let request = ListRequest::default_query().with_page_size(PageSize::All);
        let all_readings = state
            .reading_repo
            .list(ReadingFilter::for_user_book(uid, id), &request, None)
            .await
            .ok()
            .map(|page| page.items)
            .unwrap_or_default();

        let info = user_book
            .as_ref()
            .map(|ub| BookLibraryInfo::from_domain(ub, all_readings.first(), book_club));

        let active_reading_id = all_readings
            .iter()
            .find(|r| r.reading.status == crate::domain::books::readings::ReadingStatus::Reading)
            .map(|r| r.reading.id.to_string());

        let cards: Vec<BookReadingCardView> = all_readings
            .into_iter()
            .map(BookReadingCardView::from_domain)
            .collect();

        (info, cards, active_reading_id)
    } else {
        (None, Vec::new(), None)
    };

    let edit_url = format!("/books/{id}/edit");
    let view = BookDetailView::from_domain(enriched);

    let template = BookDetailTemplate {
        nav_active: "data",
        is_authenticated,
        version_info: &crate::VERSION_INFO,
        base_url: crate::base_url(),
        is_impersonating,
        impersonated_username,
        book: view,
        image_url,
        edit_url,
        library_info,
        readings,
        active_reading_id,
    };

    render_html(template).map(IntoResponse::into_response)
}

#[tracing::instrument(skip(state, cookies))]
pub(crate) async fn book_edit_page(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    Path(id): Path<crate::domain::ids::BookId>,
) -> Result<Response, StatusCode> {
    let is_authenticated = crate::application::routes::is_authenticated(&state, &cookies).await;

    if !is_authenticated {
        return Ok(Redirect::to("/login").into_response());
    }

    let (is_impersonating, impersonated_username) = impersonation_info(&state, &cookies).await;

    let enriched = state
        .book_repo
        .get_with_authors(id)
        .await
        .map_err(|e| map_app_error(e.into()))?;
    let book = enriched.book;

    let author_options: Vec<AuthorOptionView> = state
        .author_repo
        .list_all()
        .await
        .map_err(|e| map_app_error(e.into()))?
        .into_iter()
        .map(AuthorOptionView::from)
        .collect();

    let genre_options: Vec<GenreOptionView> = state
        .genre_repo
        .list_all()
        .await
        .map_err(|e| map_app_error(e.into()))?
        .into_iter()
        .map(GenreOptionView::from)
        .collect();

    let image_url = crate::application::routes::support::image_url(
        &*state.image_repo,
        "book",
        i64::from(book.id),
    )
    .await;

    let template = BookEditTemplate {
        nav_active: "data",
        is_authenticated,
        version_info: &crate::VERSION_INFO,
        is_impersonating,
        impersonated_username,
        id: book.id.to_string(),
        title: book.title,
        isbn: book.isbn.unwrap_or_default(),
        description: book.description.unwrap_or_default(),
        page_count: book.page_count.map(|p| p.to_string()).unwrap_or_default(),
        year_published: book
            .year_published
            .map(|y| y.to_string())
            .unwrap_or_default(),
        publisher: book.publisher.unwrap_or_default(),
        language: book.language.unwrap_or_default(),
        primary_genre_id: book
            .primary_genre_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        secondary_genre_id: book
            .secondary_genre_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        genre_options,
        author_options,
        author_id: enriched
            .authors
            .first()
            .map(|a| a.author_id.to_string())
            .unwrap_or_default(),
        image_url,
    };

    render_html(template).map(IntoResponse::into_response)
}

#[tracing::instrument(skip(state, cookies))]
pub(crate) async fn book_start_page(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    Path(id): Path<crate::domain::ids::BookId>,
) -> Result<Response, StatusCode> {
    let is_authenticated = crate::application::routes::is_authenticated(&state, &cookies).await;

    if !is_authenticated {
        return Ok(Redirect::to("/login").into_response());
    }

    let (is_impersonating, impersonated_username) = impersonation_info(&state, &cookies).await;

    let enriched = state
        .book_repo
        .get_with_authors(id)
        .await
        .map_err(|e| map_app_error(e.into()))?;

    let author_names = if enriched.authors.is_empty() {
        "Unknown".to_string()
    } else {
        enriched
            .authors
            .iter()
            .map(|a| a.author_name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };

    let page_count_label = enriched
        .book
        .page_count
        .map(crate::domain::formatting::format_pages)
        .unwrap_or_default();

    let thumbnail_url =
        crate::application::routes::support::image_url(&*state.image_repo, "book", i64::from(id))
            .await;

    let user_id = crate::application::routes::authenticated_user_id(&state, &cookies).await;
    let book_club = if let Some(uid) = user_id {
        state
            .user_book_repo
            .get_by_user_and_book(uid, id)
            .await
            .ok()
            .is_some_and(|ub| ub.book_club)
    } else {
        false
    };

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let template = StartBookTemplate {
        nav_active: "data",
        is_authenticated,
        version_info: &crate::VERSION_INFO,
        is_impersonating,
        impersonated_username,
        book_id: id.to_string(),
        book_label: enriched.book.title,
        author_names,
        page_count_label,
        thumbnail_url,
        started_at: today,
        book_club,
    };

    render_html(template).map(IntoResponse::into_response)
}
