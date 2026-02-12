use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::application::auth::impersonation_info;
use crate::application::errors::{AppError, map_app_error};
use crate::application::routes::render_html;
use crate::application::routes::support::{ListQuery, is_datastar_request};
use crate::application::state::AppState;
use crate::presentation::web::templates::{
    DataTemplate, GenreListTemplate, Tab, UserBookListTemplate, render_template,
};

const TABS: &[Tab] = &[
    Tab {
        key: "library",
        label: "Library",
    },
    Tab {
        key: "wishlist",
        label: "Wishlist",
    },
    Tab {
        key: "genres",
        label: "Genres",
    },
];

#[derive(Debug, Deserialize)]
pub(crate) struct DataType {
    #[serde(rename = "type", default = "default_type")]
    entity_type: String,
}

fn default_type() -> String {
    "library".to_string()
}

#[tracing::instrument(skip(state, cookies, headers, data_type, list_query))]
pub(crate) async fn data_page(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    headers: HeaderMap,
    Query(data_type): Query<DataType>,
    Query(list_query): Query<ListQuery>,
) -> Result<Response, StatusCode> {
    let entity_type = data_type.entity_type;
    let is_authenticated = crate::application::routes::is_authenticated(&state, &cookies).await;
    let user_id = crate::application::routes::authenticated_user_id(&state, &cookies).await;
    let search_value = list_query.search_value();

    let content =
        render_entity_content(&state, &entity_type, list_query, is_authenticated, user_id)
            .await
            .map_err(map_app_error)?;

    if is_datastar_request(&headers) {
        use axum::http::header::HeaderValue;
        use axum::response::Html;

        let mut response = Html(content).into_response();
        response.headers_mut().insert(
            "datastar-selector",
            HeaderValue::from_static("#data-content"),
        );
        response
            .headers_mut()
            .insert("datastar-mode", HeaderValue::from_static("inner"));
        return Ok(response);
    }

    let tabs: Vec<Tab> = TABS
        .iter()
        .map(|t| Tab {
            key: t.key,
            label: t.label,
        })
        .collect();

    let (is_impersonating, impersonated_username) = impersonation_info(&state, &cookies).await;

    let template = DataTemplate {
        nav_active: "data",
        is_authenticated,
        version_info: &crate::VERSION_INFO,
        is_impersonating,
        impersonated_username,
        active_type: entity_type,
        tabs,
        tab_signal: "_active-tab",
        tab_signal_js: "$_activeTab",
        tab_base_url: "/data?type=",
        tab_fetch_target: "#data-content",
        tab_fetch_mode: "inner",
        content,
        search_value,
    };

    render_html(template).map(IntoResponse::into_response)
}

fn render_list<T: askama::Template>(template: T, label: &str) -> Result<String, AppError> {
    render_template(template)
        .map_err(|err| AppError::unexpected(format!("failed to render {label}: {err}")))
}

async fn render_entity_content(
    state: &AppState,
    entity_type: &str,
    list_query: ListQuery,
    is_authenticated: bool,
    user_id: Option<crate::domain::ids::UserId>,
) -> Result<String, AppError> {
    match entity_type {
        "genres" => render_genres(state, list_query, is_authenticated).await,
        "wishlist" => {
            render_user_books(
                state,
                list_query,
                is_authenticated,
                user_id,
                crate::domain::user_books::Shelf::Wishlist,
            )
            .await
        }
        _ => {
            render_user_books(
                state,
                list_query,
                is_authenticated,
                user_id,
                crate::domain::user_books::Shelf::Library,
            )
            .await
        }
    }
}

async fn render_user_books(
    state: &AppState,
    list_query: ListQuery,
    is_authenticated: bool,
    user_id: Option<crate::domain::ids::UserId>,
    shelf: crate::domain::user_books::Shelf,
) -> Result<String, AppError> {
    use crate::domain::user_books::UserBookSortKey;
    let (request, search) = list_query.into_request_and_search::<UserBookSortKey>();

    let shelf_str = shelf.as_str();
    let page_path = format!("/data?type={shelf_str}");
    let fragment_path = format!("/data?type={shelf_str}#user-book-list");

    let page = if let Some(uid) = user_id {
        state
            .user_book_repo
            .list_by_user(uid, Some(shelf), &request, search.as_deref())
            .await
            .map_err(AppError::from)?
    } else {
        crate::domain::listing::Page::new(
            Vec::new(),
            1,
            crate::domain::listing::DEFAULT_PAGE_SIZE,
            0,
            false,
        )
    };

    let (user_books, navigator) = crate::application::routes::support::build_page_view(
        page,
        request,
        crate::presentation::web::views::UserBookView::from_domain,
        page_path,
        fragment_path,
        search,
    );

    render_list(
        UserBookListTemplate {
            is_authenticated,
            user_books,
            navigator,
            shelf: shelf_str,
        },
        shelf_str,
    )
}

async fn render_genres(
    state: &AppState,
    list_query: ListQuery,
    is_authenticated: bool,
) -> Result<String, AppError> {
    use crate::domain::books::genres::GenreSortKey;
    let (request, search) = list_query.into_request_and_search::<GenreSortKey>();

    let (genres, navigator) =
        super::super::api::books::genres::load_genre_page(state, request, search.as_deref())
            .await?;

    render_list(
        GenreListTemplate {
            is_authenticated,
            genres,
            navigator,
        },
        "genres",
    )
}
