pub(crate) mod analytics;
pub(crate) mod auth;
pub(crate) mod books;
pub(crate) mod images;
pub(crate) mod macros;
pub(crate) mod system;

// Re-exports
pub(crate) use analytics::stats;
pub(crate) use auth::{tokens, webauthn};
pub(crate) use books::{authors, books as book_routes, genres, readings, scan, user_books};
pub(crate) use system::{admin, backup};

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

use crate::application::state::AppState;

pub(super) fn router() -> axum::Router<AppState> {
    entity_routes()
        .merge(scan_routes())
        .merge(auth_admin_routes())
        .merge(image_routes())
}

fn entity_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/authors",
            get(authors::list_authors).post(authors::create_author),
        )
        .route(
            "/authors/{id}",
            get(authors::get_author)
                .put(authors::update_author)
                .delete(authors::delete_author),
        )
        .route(
            "/genres",
            get(genres::list_genres).post(genres::create_genre),
        )
        .route(
            "/genres/{id}",
            get(genres::get_genre)
                .put(genres::update_genre)
                .delete(genres::delete_genre),
        )
        .route(
            "/books",
            get(book_routes::list_books).post(book_routes::create_book),
        )
        .route(
            "/books/{id}",
            get(book_routes::get_book)
                .put(book_routes::update_book)
                .delete(book_routes::delete_book),
        )
        .route(
            "/readings",
            get(readings::list_readings).post(readings::create_reading),
        )
        .route(
            "/readings/{id}",
            get(readings::get_reading)
                .put(readings::update_reading)
                .delete(readings::delete_reading),
        )
        .route(
            "/user-books",
            get(user_books::list_user_books).post(user_books::create_user_book),
        )
        .route(
            "/user-books/{id}",
            axum::routing::put(user_books::move_user_book)
                .patch(user_books::set_book_club_user_book)
                .delete(user_books::delete_user_book),
        )
}

fn scan_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/extract-author", post(authors::extract_author))
        .route("/extract-book", post(scan::extract_book_scan))
        .route("/scan", post(scan::submit_scan))
        .route(
            "/books/{id}/fetch-covers",
            post(scan::fetch_covers_for_book),
        )
        .route("/cover-suggestions/{id}", get(scan::get_cover_suggestion))
        .route(
            "/cover-suggestions/{id}/thumbnail",
            get(scan::get_cover_suggestion_thumbnail),
        )
}

fn auth_admin_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/tokens",
            post(tokens::create_token).get(tokens::list_tokens),
        )
        .route("/tokens/{id}/revoke", post(tokens::revoke_token))
        .route("/passkeys", get(admin::list_passkeys))
        .route(
            "/passkeys/{id}",
            axum::routing::delete(admin::delete_passkey),
        )
        .route("/backup", get(backup::export_backup))
        .route(
            "/backup/restore",
            post(backup::restore_backup).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/backup/reset", post(backup::reset_database))
        .route("/admin/invite", post(admin::create_invite))
        .route(
            "/admin/impersonate/{user_id}",
            post(admin::start_impersonation),
        )
        .route("/admin/stop-impersonation", post(admin::stop_impersonation))
        .route("/stats/recompute", post(stats::recompute_stats))
        .route(
            "/timeline/rebuild",
            post(system::timeline::rebuild_timeline),
        )
}

fn image_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/{entity_type}/{id}/image",
            get(images::get_image)
                .put(images::upload_image)
                .delete(images::delete_image)
                .layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
        .route("/{entity_type}/{id}/thumbnail", get(images::get_thumbnail))
}

pub(super) fn webauthn_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/register/start", post(webauthn::register_start))
        .route("/register/finish", post(webauthn::register_finish))
        .route("/auth/start", get(webauthn::auth_start))
        .route("/auth/finish", post(webauthn::auth_finish))
        .route("/passkey/start", post(webauthn::passkey_add_start))
        .route("/passkey/finish", post(webauthn::passkey_add_finish))
}
