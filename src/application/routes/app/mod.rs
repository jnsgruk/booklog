mod add;
mod admin;
pub(super) mod auth;
mod authors;
mod books;
mod data;
pub(crate) mod genres;
mod home;
mod readings;
mod stats;
mod timeline;
mod webauthn;

use axum::response::IntoResponse;
use axum::routing::{get, post};

use crate::application::state::AppState;

/// Generate a static asset handler that serves an embedded file with cache headers.
macro_rules! static_asset_str {
    ($name:ident, $path:literal, $content_type:literal) => {
        async fn $name() -> impl IntoResponse {
            (
                [
                    ("content-type", $content_type),
                    ("cache-control", "public, max-age=604800"),
                ],
                include_str!($path),
            )
        }
    };
}

/// Generate a static asset handler for binary files (e.g. images).
macro_rules! static_asset_bytes {
    ($name:ident, $path:literal, $content_type:literal) => {
        async fn $name() -> impl IntoResponse {
            (
                [
                    ("content-type", $content_type),
                    ("cache-control", "public, max-age=604800"),
                ],
                include_bytes!($path).as_slice(),
            )
        }
    };
}

pub(super) fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(home::home_page))
        .route("/login", get(auth::login_page))
        .route("/logout", post(auth::logout))
        .route("/admin", get(admin::admin_page))
        .route("/register/{token}", get(webauthn::register_page))
        .route("/auth/cli-callback", get(webauthn::cli_callback_page))
        .route("/data", get(data::data_page))
        .route("/add", get(add::add_page))
        .route("/timeline", get(timeline::timeline_page))
        .route("/stats", get(stats::stats_page))
        .route("/authors/{id}", get(authors::author_detail_page))
        .route("/authors/{id}/edit", get(authors::author_edit_page))
        .route("/books/{id}", get(books::book_detail_page))
        .route("/books/{id}/edit", get(books::book_edit_page))
        .route("/books/{id}/start", get(books::book_start_page))
        .route("/readings/{id}", get(readings::reading_detail_page))
        .route("/readings/{id}/edit", get(readings::reading_edit_page))
        .route("/genres/{id}", get(genres::genre_detail_page))
        .route("/genres/{id}/edit", get(genres::genre_edit_page))
        .route("/static/css/styles.css", get(styles))
        .route("/static/js/webauthn.js", get(webauthn_js))
        .route(
            "/static/js/components/photo-capture.js",
            get(photo_capture_js),
        )
        .route(
            "/static/js/components/searchable-select.js",
            get(searchable_select_js),
        )
        .route("/static/js/components/chip-scroll.js", get(chip_scroll_js))
        .route("/static/js/location.js", get(location_js))
        .route("/static/js/image-utils.js", get(image_utils_js))
        .route("/static/js/components/donut-chart.js", get(donut_chart_js))
        .route("/static/js/components/bar-chart.js", get(bar_chart_js))
        .route(
            "/static/js/components/image-upload.js",
            get(image_upload_js),
        )
        .route("/static/favicon-light.svg", get(favicon_light))
        .route("/static/favicon-dark.svg", get(favicon_dark))
        .route("/static/og-image.png", get(og_image))
        .route("/health", get(health))
}

static_asset_str!(
    styles,
    "../../../../static/css/styles.css",
    "text/css; charset=utf-8"
);
static_asset_str!(
    webauthn_js,
    "../../../../static/js/webauthn.js",
    "application/javascript; charset=utf-8"
);
static_asset_str!(
    location_js,
    "../../../../static/js/location.js",
    "application/javascript; charset=utf-8"
);
static_asset_str!(
    image_utils_js,
    "../../../../static/js/image-utils.js",
    "application/javascript; charset=utf-8"
);
static_asset_str!(
    photo_capture_js,
    "../../../../static/js/components/photo-capture.js",
    "application/javascript; charset=utf-8"
);
static_asset_str!(
    searchable_select_js,
    "../../../../static/js/components/searchable-select.js",
    "application/javascript; charset=utf-8"
);
static_asset_str!(
    chip_scroll_js,
    "../../../../static/js/components/chip-scroll.js",
    "application/javascript; charset=utf-8"
);
static_asset_str!(
    donut_chart_js,
    "../../../../static/js/components/donut-chart.js",
    "application/javascript; charset=utf-8"
);
static_asset_str!(
    bar_chart_js,
    "../../../../static/js/components/bar-chart.js",
    "application/javascript; charset=utf-8"
);
static_asset_str!(
    image_upload_js,
    "../../../../static/js/components/image-upload.js",
    "application/javascript; charset=utf-8"
);
static_asset_str!(
    favicon_light,
    "../../../../static/favicon-light.svg",
    "image/svg+xml"
);
static_asset_str!(
    favicon_dark,
    "../../../../static/favicon-dark.svg",
    "image/svg+xml"
);
static_asset_bytes!(og_image, "../../../../static/og-image.png", "image/png");

async fn health() -> impl IntoResponse {
    ([("content-type", "application/json")], r#"{"status":"ok"}"#)
}
