pub mod api;
pub mod app;
pub mod support;

pub(crate) use app::auth::{authenticated_user_id, is_authenticated};

use askama::Template;
use axum::http::{HeaderValue, Request, StatusCode};
use axum::response::Html;
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::compression::CompressionLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::{DefaultOnResponse, MakeSpan, TraceLayer};
use tracing::{Level, Span, error};

use crate::application::rate_limit::RateLimitLayer;
use crate::application::state::AppState;

use crate::presentation::web::templates::render_template;

/// 5 MB request body limit.
const BODY_LIMIT_BYTES: usize = 5 * 1024 * 1024;

/// Maximum auth requests per IP per minute.
const AUTH_RATE_LIMIT_PER_MINUTE: u32 = 10;

pub fn app_router(state: AppState) -> axum::Router {
    axum::Router::new()
        .merge(app::router())
        .nest("/api/v1", api::router())
        .nest(
            "/api/v1/webauthn",
            api::webauthn_router().layer(RateLimitLayer::per_minute(AUTH_RATE_LIMIT_PER_MINUTE)),
        )
        .layer(
            ServiceBuilder::new()
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(BooklogMakeSpan)
                        .on_response(DefaultOnResponse::new().level(Level::INFO)),
                )
                .layer(CookieManagerLayer::new())
                .layer(RequestBodyLimitLayer::new(BODY_LIMIT_BYTES))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::X_CONTENT_TYPE_OPTIONS,
                    HeaderValue::from_static("nosniff"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::X_FRAME_OPTIONS,
                    HeaderValue::from_static("DENY"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::REFERRER_POLICY,
                    HeaderValue::from_static("strict-origin-when-cross-origin"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::CONTENT_SECURITY_POLICY,
                    HeaderValue::from_static(
                        "default-src 'self'; \
                         script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.jsdelivr.net; \
                         style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
                         font-src 'self' https://fonts.gstatic.com; \
                         img-src 'self' data: blob:; \
                         frame-ancestors 'none'",
                    ),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::STRICT_TRANSPORT_SECURITY,
                    HeaderValue::from_static("max-age=63072000; includeSubDomains"),
                ))
                .layer(CompressionLayer::new().gzip(true)),
        )
        .with_state(state)
}

#[derive(Clone)]
struct BooklogMakeSpan;

impl<B> MakeSpan<B> for BooklogMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        tracing::info_span!(
            "request",
            method = %request.method(),
            uri = %request.uri(),
            version = ?request.version(),
            user.id = tracing::field::Empty,
        )
    }
}

pub(crate) fn render_html<T: Template>(template: T) -> Result<Html<String>, StatusCode> {
    render_template(template).map(Html).map_err(|err| {
        error!(error = %err, "failed to render template");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
