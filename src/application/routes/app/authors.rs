use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};

use crate::application::auth::impersonation_info;
use crate::application::errors::map_app_error;
use crate::application::routes::render_html;
use crate::application::state::AppState;
use crate::presentation::web::templates::{AuthorDetailTemplate, AuthorEditTemplate};
use crate::presentation::web::views::{AuthorBookCardView, AuthorDetailView};

#[tracing::instrument(skip(state, cookies))]
pub(crate) async fn author_detail_page(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    Path(id): Path<crate::domain::ids::AuthorId>,
) -> Result<Response, StatusCode> {
    let is_authenticated = crate::application::routes::is_authenticated(&state, &cookies).await;
    let user_id = crate::application::routes::authenticated_user_id(&state, &cookies).await;
    let (is_impersonating, impersonated_username) = impersonation_info(&state, &cookies).await;

    let author = state
        .author_repo
        .get(id)
        .await
        .map_err(|e| map_app_error(e.into()))?;

    let image_url = crate::application::routes::support::image_url(
        &*state.image_repo,
        "author",
        i64::from(author.id),
    )
    .await;

    let library_books = load_author_library_books(&state, id, user_id)
        .await
        .map_err(map_app_error)?;

    let edit_url = format!("/authors/{}/edit", author.id);
    let view = AuthorDetailView::from_domain(author);

    let template = AuthorDetailTemplate {
        nav_active: "data",
        is_authenticated,
        version_info: &crate::VERSION_INFO,
        base_url: crate::base_url(),
        is_impersonating,
        impersonated_username,
        author: view,
        image_url,
        edit_url,
        library_books,
    };

    render_html(template).map(IntoResponse::into_response)
}

async fn load_author_library_books(
    state: &AppState,
    author_id: crate::domain::ids::AuthorId,
    user_id: Option<crate::domain::ids::UserId>,
) -> Result<Vec<AuthorBookCardView>, crate::application::errors::AppError> {
    let Some(uid) = user_id else {
        return Ok(Vec::new());
    };

    let author_books = state
        .book_repo
        .list_by_author(author_id)
        .await
        .map_err(crate::application::errors::AppError::from)?;

    let user_book_ids = state
        .user_book_repo
        .book_ids_for_user(uid, None)
        .await
        .map_err(crate::application::errors::AppError::from)?;

    let library_books: Vec<_> = author_books
        .into_iter()
        .filter(|bwa| user_book_ids.contains(&bwa.book.id))
        .collect();

    let book_ids: Vec<i64> = library_books
        .iter()
        .map(|bwa| i64::from(bwa.book.id))
        .collect();
    let books_with_images: std::collections::HashSet<i64> = state
        .image_repo
        .entity_ids_with_images("book", &book_ids)
        .await
        .unwrap_or_default();

    let cards: Vec<AuthorBookCardView> = library_books
        .into_iter()
        .map(|bwa| {
            let has_image = books_with_images.contains(&i64::from(bwa.book.id));
            let book_id_str = bwa.book.id.to_string();
            let mut card = AuthorBookCardView::from_domain(bwa);
            if has_image {
                card.thumbnail_url = Some(format!("/api/v1/book/{book_id_str}/thumbnail"));
            }
            card
        })
        .collect();

    Ok(cards)
}

#[tracing::instrument(skip(state, cookies))]
pub(crate) async fn author_edit_page(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    Path(id): Path<crate::domain::ids::AuthorId>,
) -> Result<Response, StatusCode> {
    let is_authenticated = crate::application::routes::is_authenticated(&state, &cookies).await;

    if !is_authenticated {
        return Ok(Redirect::to("/login").into_response());
    }

    let (is_impersonating, impersonated_username) = impersonation_info(&state, &cookies).await;

    let author = state
        .author_repo
        .get(id)
        .await
        .map_err(|e| map_app_error(e.into()))?;

    let image_url = crate::application::routes::support::image_url(
        &*state.image_repo,
        "author",
        i64::from(author.id),
    )
    .await;

    let template = AuthorEditTemplate {
        nav_active: "data",
        is_authenticated,
        version_info: &crate::VERSION_INFO,
        is_impersonating,
        impersonated_username,
        id: author.id.to_string(),
        name: author.name,
        image_url,
    };

    render_html(template).map(IntoResponse::into_response)
}
