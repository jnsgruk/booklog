use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::application::errors::{AppError, map_app_error};
use crate::application::routes::render_html;
use crate::application::state::AppState;
use crate::domain::ids::UserId;
use crate::domain::listing::{ListRequest, PageSize, SortDirection, SortKey};
use crate::domain::readings::{ReadingFilter, ReadingSortKey, ReadingStatus};
use crate::domain::timeline::TimelineSortKey;
use crate::domain::user_books::{Shelf, UserBookSortKey};
use rand::seq::SliceRandom;

use crate::application::auth::impersonation_info;
use crate::domain::stats::CachedStats;
use crate::presentation::web::templates::HomeTemplate;
use crate::presentation::web::views::{
    ReadingView, StatCard, StatsView, TimelineEventView, UserBookView,
};

#[tracing::instrument(skip(state, cookies))]
pub(crate) async fn home_page(
    State(state): State<AppState>,
    cookies: tower_cookies::Cookies,
) -> Result<Response, StatusCode> {
    let user_id = crate::application::routes::authenticated_user_id(&state, &cookies).await;
    let is_authenticated = user_id.is_some();

    let (content, stats_view) = tokio::try_join!(
        load_home_content(&state, user_id),
        load_stats(&state, user_id),
    )
    .map_err(map_app_error)?;

    let stat_cards = if let Some(uid) = user_id {
        state
            .stats_repo
            .get_cached(uid)
            .await
            .ok()
            .flatten()
            .map(build_stat_cards)
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let (is_impersonating, impersonated_username) = impersonation_info(&state, &cookies).await;

    let template = HomeTemplate {
        nav_active: "home",
        is_authenticated,
        version_info: &crate::VERSION_INFO,
        base_url: crate::base_url(),
        is_impersonating,
        impersonated_username,
        currently_reading: content.currently_reading,
        recently_added: content.recently_added,
        wishlist: content.wishlist,
        recent_events: content.recent_events,
        stats: stats_view,
        stat_cards,
    };

    render_html(template).map(IntoResponse::into_response)
}

struct HomeContent {
    currently_reading: Vec<ReadingView>,
    recently_added: Vec<UserBookView>,
    wishlist: Vec<UserBookView>,
    recent_events: Vec<TimelineEventView>,
}

/// Build a `ListRequest` that fetches page 1 with 1 item, using a sort key's
/// defaults. Used to obtain `Page.total` for entity counts.
fn count_request<K: SortKey>() -> ListRequest<K> {
    let key = K::default();
    ListRequest::new(1, PageSize::limited(1), key, key.default_direction())
}

async fn load_home_content(
    state: &AppState,
    user_id: Option<UserId>,
) -> Result<HomeContent, AppError> {
    let (reading_page, recent_events_page, recently_added_page, wishlist_page) =
        fetch_home_pages(state, user_id).await?;

    let mut currently_reading: Vec<ReadingView> = reading_page
        .items
        .into_iter()
        .map(ReadingView::from_domain)
        .collect();

    let mut recently_added: Vec<UserBookView> = recently_added_page
        .items
        .into_iter()
        .map(UserBookView::from_domain)
        .collect();

    let mut wishlist: Vec<UserBookView> = wishlist_page
        .items
        .into_iter()
        .map(UserBookView::from_domain)
        .collect();

    // Batch-check which books have cover images and set thumbnail URLs
    let all_book_ids: Vec<i64> = currently_reading
        .iter()
        .filter_map(|v| v.book_id.parse().ok())
        .chain(recently_added.iter().filter_map(|v| v.book_id.parse().ok()))
        .chain(wishlist.iter().filter_map(|v| v.book_id.parse().ok()))
        .collect();
    let books_with_images: std::collections::HashSet<i64> = state
        .image_repo
        .entity_ids_with_images("book", &all_book_ids)
        .await
        .unwrap_or_default();

    for view in &mut currently_reading {
        enrich_thumbnail(&view.book_id, &mut view.thumbnail_url, &books_with_images);
    }
    for view in &mut recently_added {
        enrich_thumbnail(&view.book_id, &mut view.thumbnail_url, &books_with_images);
    }
    for view in &mut wishlist {
        enrich_thumbnail(&view.book_id, &mut view.thumbnail_url, &books_with_images);
    }

    let recent_events = recent_events_page
        .items
        .into_iter()
        .map(TimelineEventView::from_domain)
        .collect();

    Ok(HomeContent {
        currently_reading,
        recently_added,
        wishlist,
        recent_events,
    })
}

type HomePages = (
    crate::domain::listing::Page<crate::domain::readings::ReadingWithBook>,
    crate::domain::listing::Page<crate::domain::timeline::TimelineEvent>,
    crate::domain::listing::Page<crate::domain::user_books::UserBookWithDetails>,
    crate::domain::listing::Page<crate::domain::user_books::UserBookWithDetails>,
);

async fn fetch_home_pages(
    state: &AppState,
    user_id: Option<UserId>,
) -> Result<HomePages, AppError> {
    use crate::domain::listing::Page as DomainPage;

    let reading_req = ListRequest::new(
        1,
        PageSize::limited(10),
        ReadingSortKey::CreatedAt,
        SortDirection::Desc,
    );
    let events_req = ListRequest::new(
        1,
        PageSize::limited(5),
        TimelineSortKey::default(),
        TimelineSortKey::default().default_direction(),
    );
    let added_req = ListRequest::new(
        1,
        PageSize::limited(10),
        UserBookSortKey::CreatedAt,
        SortDirection::Desc,
    );
    let wishlist_req = ListRequest::new(
        1,
        PageSize::limited(10),
        UserBookSortKey::CreatedAt,
        SortDirection::Desc,
    );

    let empty_reading = || DomainPage::new(Vec::new(), 1, 10, 0, false);
    let empty_user_book = || DomainPage::new(Vec::new(), 1, 10, 0, false);

    tokio::try_join!(
        async {
            if let Some(uid) = user_id {
                state
                    .reading_repo
                    .list(
                        ReadingFilter::for_user_status(uid, ReadingStatus::Reading),
                        &reading_req,
                        None,
                    )
                    .await
                    .map_err(AppError::from)
            } else {
                Ok(empty_reading())
            }
        },
        async {
            state
                .timeline_repo
                .list(user_id, &events_req)
                .await
                .map_err(AppError::from)
        },
        async {
            if let Some(uid) = user_id {
                state
                    .user_book_repo
                    .list_by_user(uid, Some(Shelf::Library), &added_req, None)
                    .await
                    .map_err(AppError::from)
            } else {
                Ok(empty_user_book())
            }
        },
        async {
            if let Some(uid) = user_id {
                state
                    .user_book_repo
                    .list_by_user(uid, Some(Shelf::Wishlist), &wishlist_req, None)
                    .await
                    .map_err(AppError::from)
            } else {
                Ok(empty_user_book())
            }
        },
    )
}

fn enrich_thumbnail(
    book_id: &str,
    thumbnail_url: &mut Option<String>,
    books_with_images: &std::collections::HashSet<i64>,
) {
    if let Ok(id) = book_id.parse::<i64>()
        && books_with_images.contains(&id)
    {
        *thumbnail_url = Some(format!("/api/v1/book/{book_id}/thumbnail"));
    }
}

async fn load_stats(state: &AppState, user_id: Option<UserId>) -> Result<StatsView, AppError> {
    let Some(uid) = user_id else {
        return Ok(StatsView {
            library: 0,
            wishlist: 0,
            currently_reading: 0,
        });
    };

    let user_book_req: ListRequest<UserBookSortKey> = count_request();
    let reading_req: ListRequest<ReadingSortKey> = count_request();

    let (library, wishlist, currently_reading) = tokio::try_join!(
        async {
            state
                .user_book_repo
                .list_by_user(uid, Some(Shelf::Library), &user_book_req, None)
                .await
                .map_err(AppError::from)
        },
        async {
            state
                .user_book_repo
                .list_by_user(uid, Some(Shelf::Wishlist), &user_book_req, None)
                .await
                .map_err(AppError::from)
        },
        async {
            state
                .reading_repo
                .list(
                    ReadingFilter::for_user_status(uid, ReadingStatus::Reading),
                    &reading_req,
                    None,
                )
                .await
                .map_err(AppError::from)
        },
    )?;

    Ok(StatsView {
        library: library.total,
        wishlist: wishlist.total,
        currently_reading: currently_reading.total,
    })
}

fn build_stat_cards(cs: CachedStats) -> Vec<StatCard> {
    let mut cards = vec![
        StatCard {
            icon: "book",
            value: cs.reading.books_last_30_days.to_string(),
            label: "Books (30d)",
        },
        StatCard {
            icon: "book",
            value: cs.reading.books_all_time.to_string(),
            label: "All Time",
        },
        StatCard {
            icon: "book",
            value: cs.reading.pages_all_time.to_string(),
            label: "Pages Read",
        },
        StatCard {
            icon: "map",
            value: cs.book_summary.unique_genres.to_string(),
            label: "Genres",
        },
        StatCard {
            icon: "book",
            value: cs
                .book_summary
                .top_genre
                .unwrap_or_else(|| crate::domain::formatting::EM_DASH.into()),
            label: "Top Genre",
        },
        StatCard {
            icon: "book",
            value: cs
                .book_summary
                .top_author
                .unwrap_or_else(|| crate::domain::formatting::EM_DASH.into()),
            label: "Top Author",
        },
    ];
    cards.shuffle(&mut rand::thread_rng());
    cards
}
