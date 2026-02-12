use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::application::auth::impersonation_info;
use crate::application::errors::map_app_error;
use crate::application::routes::render_html;
use crate::application::routes::support::{
    load_author_options, load_book_options, load_genre_options,
};
use crate::application::state::AppState;
use crate::presentation::web::templates::{AddTemplate, Tab};

const ADD_TABS: &[Tab] = &[
    Tab {
        key: "author",
        label: "Author",
    },
    Tab {
        key: "genre",
        label: "Genre",
    },
    Tab {
        key: "book",
        label: "Book",
    },
    Tab {
        key: "reading",
        label: "Library",
    },
];

#[derive(Debug, Deserialize)]
pub(crate) struct AddQuery {
    #[serde(rename = "type", default = "default_type")]
    entity_type: String,
    book_id: Option<String>,
    status: Option<String>,
}

fn default_type() -> String {
    "author".to_string()
}

#[tracing::instrument(skip(state, cookies))]
pub(crate) async fn add_page(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    Query(query): Query<AddQuery>,
) -> Result<Response, StatusCode> {
    let is_authenticated = crate::application::routes::is_authenticated(&state, &cookies).await;

    if !is_authenticated {
        return Ok(Redirect::to("/login").into_response());
    }

    let (genre_options, author_options, book_options) = tokio::try_join!(
        async { load_genre_options(&state).await },
        async { load_author_options(&state).await },
        async { load_book_options(&state).await },
    )
    .map_err(map_app_error)?;

    let (is_impersonating, impersonated_username) = impersonation_info(&state, &cookies).await;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let template = AddTemplate {
        nav_active: "data",
        is_authenticated,
        version_info: &crate::VERSION_INFO,
        is_impersonating,
        impersonated_username,
        active_type: query.entity_type,
        tabs: ADD_TABS
            .iter()
            .map(|t| Tab {
                key: t.key,
                label: t.label,
            })
            .collect(),
        tab_signal: "_add-type",
        tab_signal_js: "$_addType",
        tab_base_url: "",
        tab_fetch_target: "",
        tab_fetch_mode: "",
        genre_options,
        author_options,
        book_options,
        selected_book_id: query.book_id.unwrap_or_default(),
        selected_status: query.status.unwrap_or_default(),
        today,
    };

    render_html(template).map(IntoResponse::into_response)
}
