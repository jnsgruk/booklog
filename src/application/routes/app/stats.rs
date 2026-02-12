use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header::HeaderValue};
use axum::response::{Html, IntoResponse, Response};
use chrono::Utc;
use serde::Deserialize;

use crate::application::auth::impersonation_info;
use crate::application::errors::map_app_error;
use crate::application::routes::render_html;
use crate::application::routes::support::is_datastar_request;
use crate::application::services::stats::{compute_all_stats, compute_stats_for_year};
use crate::application::state::AppState;
use crate::domain::ids::UserId;
use crate::domain::stats::{BookSummaryStats, CachedStats, ReadingStats};
use crate::presentation::web::templates::{
    StatsContentTemplate, StatsPageTemplate, YearTab, render_template,
};

#[derive(Debug, Deserialize)]
pub(crate) struct StatsQuery {
    year: Option<String>,
}

#[tracing::instrument(skip(state, cookies, headers, query))]
pub(crate) async fn stats_page(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
    headers: HeaderMap,
    Query(query): Query<StatsQuery>,
) -> Result<Response, StatusCode> {
    let user_id = crate::application::routes::authenticated_user_id(&state, &cookies).await;
    let is_authenticated = user_id.is_some();

    let year_filter = parse_year_filter(query.year.as_ref());

    let cached = match year_filter {
        Some(year) => compute_year_stats(&state, user_id, year).await?,
        None => load_or_compute(&state, user_id).await?,
    };

    let is_year_view = year_filter.is_some();
    let content_html = render_stats_content(&cached, is_year_view)?;

    // Datastar fragment request: return the content wrapped in its container
    // and use `replace` mode so the entire element is swapped via
    // `replaceWith()`. The `inner` mode's DOM-morphing algorithm interferes
    // with web components (bar-chart, donut-chart) that render their own
    // children — the morpher tries to reconcile the component's rendered SVG
    // against the empty server-sent element, preventing the chart from
    // appearing until a full page reload.
    if is_datastar_request(&headers) {
        let wrapped =
            format!(r#"<div id="stats-content" class="flex flex-col gap-8">{content_html}</div>"#);
        let mut response = Html(wrapped).into_response();
        response.headers_mut().insert(
            "datastar-selector",
            HeaderValue::from_static("#stats-content"),
        );
        response
            .headers_mut()
            .insert("datastar-mode", HeaderValue::from_static("replace"));
        return Ok(response);
    }

    // Full page request
    let cache_age = if year_filter.is_none() {
        format_cache_age(&cached.computed_at)
    } else {
        String::new()
    };

    let year_tabs = build_year_tabs(&state, user_id).await;
    let active_year = year_filter.map_or_else(|| "all".to_string(), |y| y.to_string());

    let (is_impersonating, impersonated_username) = impersonation_info(&state, &cookies).await;

    let template = StatsPageTemplate {
        nav_active: "stats",
        is_authenticated,
        version_info: &crate::VERSION_INFO,
        base_url: crate::base_url(),
        is_impersonating,
        impersonated_username,
        cache_age,
        content: content_html,
        year_tabs,
        active_year,
    };

    render_html(template).map(IntoResponse::into_response)
}

/// Parse the year query parameter, treating "all" or absent as None.
fn parse_year_filter(year: Option<&String>) -> Option<i32> {
    year.and_then(|s| {
        if s == "all" {
            return None;
        }
        s.parse::<i32>()
            .ok()
            .filter(|&y| (2000..=2100).contains(&y))
    })
}

/// Build year tabs from available years.
async fn build_year_tabs(state: &AppState, user_id: Option<UserId>) -> Vec<YearTab> {
    let Some(uid) = user_id else {
        return vec![];
    };

    let years = state
        .stats_repo
        .available_years(uid)
        .await
        .unwrap_or_default();

    if years.is_empty() {
        return vec![];
    }

    let mut tabs = vec![YearTab {
        key: "all".to_string(),
        label: "All Time".to_string(),
    }];

    for year in years {
        tabs.push(YearTab {
            key: year.to_string(),
            label: year.to_string(),
        });
    }

    tabs
}

/// Load stats from cache, falling back to live computation on cache miss.
async fn load_or_compute(
    state: &AppState,
    user_id: Option<UserId>,
) -> Result<CachedStats, StatusCode> {
    if let Some(uid) = user_id {
        if let Ok(Some(cached)) = state.stats_repo.get_cached(uid).await {
            return Ok(cached);
        }
        tracing::debug!("stats cache miss, computing live");
        compute_all_stats(&*state.stats_repo, uid)
            .await
            .map_err(|e| map_app_error(e.into()))
    } else {
        Ok(CachedStats {
            book_summary: BookSummaryStats::default(),
            reading: ReadingStats::default(),
            computed_at: chrono::Utc::now().to_rfc3339(),
        })
    }
}

/// Compute year-specific stats on demand (no caching).
async fn compute_year_stats(
    state: &AppState,
    user_id: Option<UserId>,
    year: i32,
) -> Result<CachedStats, StatusCode> {
    if let Some(uid) = user_id {
        compute_stats_for_year(&*state.stats_repo, uid, year)
            .await
            .map_err(|e| map_app_error(e.into()))
    } else {
        Ok(CachedStats {
            book_summary: BookSummaryStats::default(),
            reading: ReadingStats::default(),
            computed_at: chrono::Utc::now().to_rfc3339(),
        })
    }
}

/// Render the stats content HTML from `CachedStats`.
fn render_stats_content(cached: &CachedStats, is_year_view: bool) -> Result<String, StatusCode> {
    let has_data = cached.reading.books_all_time > 0 || cached.book_summary.unique_genres > 0;
    let rating_chart_data = build_rating_chart_data(&cached.reading.rating_distribution);
    let activity_chart_data = if is_year_view {
        build_activity_chart_data(&cached.reading.monthly_books, &cached.reading.monthly_pages)
    } else {
        build_activity_chart_data(&cached.reading.yearly_books, &cached.reading.yearly_pages)
    };
    let pages_formatted = format_number(cached.reading.pages_all_time);

    let avg_rating_formatted = cached.reading.average_rating.map_or_else(
        || crate::domain::formatting::EM_DASH.to_string(),
        |r| format!("{r:.1}/5"),
    );

    let avg_days_formatted = cached.reading.average_days_to_finish.map_or_else(
        || crate::domain::formatting::EM_DASH.to_string(),
        |d| {
            let days = d.round() as i64;
            if days == 1 {
                "1 day".to_string()
            } else {
                format!("{days} days")
            }
        },
    );

    render_template(StatsContentTemplate {
        book_summary: cached.book_summary.clone(),
        reading: cached.reading.clone(),
        has_data,
        is_year_view,
        activity_chart_data,
        pages_formatted,
        avg_rating_formatted,
        avg_days_formatted,
        rating_chart_data,
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Format the cache timestamp as a relative age string (e.g. "Just now", "2m ago").
fn format_cache_age(computed_at: &str) -> String {
    let Ok(ts) = chrono::DateTime::parse_from_rfc3339(computed_at) else {
        return String::new();
    };
    crate::domain::formatting::format_relative_time(ts.with_timezone(&Utc), Utc::now())
}

/// Build the pipe-separated data-items string for the activity bar chart.
/// Format: "label:books:pages|label:books:pages|..."
fn build_activity_chart_data(books: &[(String, u64)], pages: &[(String, i64)]) -> String {
    books
        .iter()
        .zip(pages.iter())
        .map(|((name, b), (_, p))| format!("{name}:{b}:{p}"))
        .collect::<Vec<_>>()
        .join("|")
}

/// Build pipe-separated data-items string for the rating donut chart.
/// Buckets half-star ratings into whole stars: 0.5+1.0→1★, 1.5+2.0→2★, etc.
/// Format: "1★:count|2★:count|..." (only non-zero buckets).
fn build_rating_chart_data(distribution: &[(f64, u64)]) -> String {
    let count_for = |r: f64| -> u64 {
        distribution
            .iter()
            .find(|(rating, _)| (*rating - r).abs() < f64::EPSILON)
            .map_or(0, |(_, c)| *c)
    };

    // Each bucket sums the half-star below and the whole star: (0.5+1.0), (1.5+2.0), etc.
    let buckets: [(i32, f64, f64); 5] = [
        (1, 0.5, 1.0),
        (2, 1.5, 2.0),
        (3, 2.5, 3.0),
        (4, 3.5, 4.0),
        (5, 4.5, 5.0),
    ];

    buckets
        .iter()
        .filter_map(|&(label, lo, hi)| {
            let count = count_for(lo) + count_for(hi);
            if count > 0 {
                Some(format!("{label}\u{2605}:{count}"))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// Format a number with thousand separators.
fn format_number(n: i64) -> String {
    if n < 1_000 {
        return n.to_string();
    }
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
