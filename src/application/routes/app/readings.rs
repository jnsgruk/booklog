use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::application::auth::impersonation_info;
use crate::application::errors::map_app_error;
use crate::application::routes::render_html;
use crate::application::state::AppState;
use crate::domain::ids::ReadingId;
use crate::presentation::web::templates::{
    FinishReadingTemplate, ReadingDetailTemplate, ReadingEditTemplate, StartReadingTemplate,
};
use crate::presentation::web::views::{BookOptionView, ReadingDetailView};

#[derive(Debug, Deserialize)]
pub(crate) struct ReadingEditQuery {
    #[serde(default)]
    start: bool,
    #[serde(default)]
    finish: bool,
}

#[tracing::instrument(skip(state, cookies))]
pub(crate) async fn reading_detail_page(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    Path(id): Path<ReadingId>,
) -> Result<Response, StatusCode> {
    let is_authenticated = crate::application::routes::is_authenticated(&state, &cookies).await;
    let (is_impersonating, impersonated_username) = impersonation_info(&state, &cookies).await;

    let reading = state
        .reading_repo
        .get_with_book(id)
        .await
        .map_err(|e| map_app_error(e.into()))?;

    let edit_url = format!("/readings/{id}/edit");
    let view = ReadingDetailView::from_domain(reading);

    let template = ReadingDetailTemplate {
        nav_active: "data",
        is_authenticated,
        version_info: &crate::VERSION_INFO,
        base_url: crate::base_url(),
        is_impersonating,
        impersonated_username,
        reading: view,
        edit_url,
    };

    render_html(template).map(IntoResponse::into_response)
}

#[tracing::instrument(skip(state, cookies))]
pub(crate) async fn reading_edit_page(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    Path(id): Path<ReadingId>,
    Query(query): Query<ReadingEditQuery>,
) -> Result<Response, StatusCode> {
    let is_authenticated = crate::application::routes::is_authenticated(&state, &cookies).await;

    if !is_authenticated {
        return Ok(Redirect::to("/login").into_response());
    }

    let (is_impersonating, impersonated_username) = impersonation_info(&state, &cookies).await;

    let reading = state
        .reading_repo
        .get_with_book(id)
        .await
        .map_err(|e| map_app_error(e.into()))?;

    let format = reading
        .reading
        .format
        .map(|f| f.as_str().to_string())
        .unwrap_or_default();

    if query.start || query.finish {
        return render_start_or_finish(
            &state,
            reading,
            format,
            query.start,
            is_authenticated,
            is_impersonating,
            impersonated_username,
        )
        .await;
    }

    let user_id = crate::application::routes::authenticated_user_id(&state, &cookies).await;
    let is_book_club = if let Some(uid) = user_id {
        state
            .user_book_repo
            .get_by_user_and_book(uid, reading.reading.book_id)
            .await
            .ok()
            .is_some_and(|ub| ub.book_club)
    } else {
        false
    };

    let book_options: Vec<BookOptionView> = state
        .book_repo
        .list_all()
        .await
        .map_err(|e| map_app_error(e.into()))?
        .into_iter()
        .map(BookOptionView::from)
        .collect();

    let template = ReadingEditTemplate {
        nav_active: "data",
        is_authenticated,
        version_info: &crate::VERSION_INFO,
        is_impersonating,
        impersonated_username,
        id: reading.reading.id.to_string(),
        book_id: reading.reading.book_id.to_string(),
        book_label: reading.book_title,
        status: reading.reading.status.as_str().to_string(),
        format,
        started_at: reading
            .reading
            .started_at
            .map(|d| d.to_string())
            .unwrap_or_default(),
        finished_at: reading
            .reading
            .finished_at
            .map(|d| d.to_string())
            .unwrap_or_default(),
        rating: reading
            .reading
            .rating
            .map(|r| r.to_string())
            .unwrap_or_default(),
        quick_reviews: reading
            .reading
            .quick_reviews
            .iter()
            .map(|r| r.form_value())
            .collect::<Vec<_>>()
            .join(","),
        book_club: is_book_club,
        book_options,
    };

    render_html(template).map(IntoResponse::into_response)
}

async fn render_start_or_finish(
    state: &AppState,
    reading: crate::domain::readings::ReadingWithBook,
    format: String,
    is_start: bool,
    is_authenticated: bool,
    is_impersonating: bool,
    impersonated_username: String,
) -> Result<Response, StatusCode> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let book_id = reading.reading.book_id;
    let author_names = if reading.author_names.is_empty() {
        "Unknown".to_string()
    } else {
        reading.author_names.clone()
    };
    let page_count_label = reading
        .page_count
        .map(crate::domain::formatting::format_pages)
        .unwrap_or_default();
    let thumbnail_url = crate::application::routes::support::image_url(
        &*state.image_repo,
        "book",
        i64::from(book_id),
    )
    .await;

    if is_start {
        let template = StartReadingTemplate {
            nav_active: "data",
            is_authenticated,
            version_info: &crate::VERSION_INFO,
            is_impersonating,
            impersonated_username,
            id: reading.reading.id.to_string(),
            book_id: book_id.to_string(),
            book_label: reading.book_title,
            author_names,
            page_count_label,
            thumbnail_url,
            status: "reading".to_string(),
            format,
            started_at: today,
        };
        return render_html(template).map(IntoResponse::into_response);
    }

    let template = FinishReadingTemplate {
        nav_active: "data",
        is_authenticated,
        version_info: &crate::VERSION_INFO,
        is_impersonating,
        impersonated_username: impersonated_username.clone(),
        id: reading.reading.id.to_string(),
        book_id: book_id.to_string(),
        book_label: reading.book_title,
        author_names,
        page_count_label,
        thumbnail_url,
        status: "read".to_string(),
        format,
        started_at: reading
            .reading
            .started_at
            .map(|d| d.to_string())
            .unwrap_or_default(),
        finished_at: today,
        rating: reading
            .reading
            .rating
            .map(|r| r.to_string())
            .unwrap_or_default(),
        quick_reviews: reading
            .reading
            .quick_reviews
            .iter()
            .map(|r| r.form_value())
            .collect::<Vec<_>>()
            .join(","),
    };
    render_html(template).map(IntoResponse::into_response)
}
