use askama::Template;

use super::views::{
    AuthorBookCardView, AuthorDetailView, AuthorOptionView, AuthorView, BookDetailView,
    BookLibraryInfo, BookOptionView, BookReadingCardView, BookView, GenreDetailView,
    GenreOptionView, GenreView, ListNavigator, Paginated, ReadingDetailView, ReadingView, StatCard,
    StatsView, TimelineEventView, TimelineMonthView, UserBookView,
};
use crate::domain::analytics::stats::{BookSummaryStats, ReadingStats};
use crate::domain::analytics::timeline::TimelineSortKey;
use crate::domain::books::authors::AuthorSortKey;
use crate::domain::books::books::BookSortKey;
use crate::domain::books::genres::GenreSortKey;
use crate::domain::books::readings::ReadingSortKey;
use crate::domain::books::user_books::UserBookSortKey;

#[derive(Template)]
#[template(path = "partials/lists/author_list.html")]
pub struct AuthorListTemplate {
    pub is_authenticated: bool,
    pub authors: Paginated<AuthorView>,
    pub navigator: ListNavigator<AuthorSortKey>,
}

#[derive(Template)]
#[template(path = "partials/lists/book_list.html")]
pub struct BookListTemplate {
    pub is_authenticated: bool,
    pub books: Paginated<BookView>,
    pub navigator: ListNavigator<BookSortKey>,
}

#[derive(Template)]
#[template(path = "partials/lists/reading_list.html")]
pub struct ReadingListTemplate {
    pub is_authenticated: bool,
    pub readings: Paginated<ReadingView>,
    pub navigator: ListNavigator<ReadingSortKey>,
}

#[derive(Template)]
#[template(path = "partials/lists/user_book_list.html")]
pub struct UserBookListTemplate {
    pub is_authenticated: bool,
    pub user_books: Paginated<UserBookView>,
    pub navigator: ListNavigator<UserBookSortKey>,
    pub shelf: &'static str,
}

#[derive(Template)]
#[template(path = "partials/lists/genre_list.html")]
pub struct GenreListTemplate {
    pub is_authenticated: bool,
    pub genres: Paginated<GenreView>,
    pub navigator: ListNavigator<GenreSortKey>,
}

#[derive(Template)]
#[template(path = "pages/timeline.html")]
pub struct TimelineTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub is_impersonating: bool,
    pub impersonated_username: String,

    pub events: Paginated<TimelineEventView>,
    pub navigator: ListNavigator<TimelineSortKey>,
    pub months: Vec<TimelineMonthView>,
}

#[derive(Template)]
#[template(path = "partials/timeline_chunk.html")]
pub struct TimelineChunkTemplate {
    pub is_authenticated: bool,
    pub events: Paginated<TimelineEventView>,
    pub navigator: ListNavigator<TimelineSortKey>,
    pub months: Vec<TimelineMonthView>,
}

#[derive(Template)]
#[template(path = "pages/home.html")]
pub struct HomeTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub base_url: &'static str,
    pub is_impersonating: bool,
    pub impersonated_username: String,

    pub currently_reading: Vec<ReadingView>,
    pub recently_added: Vec<UserBookView>,
    pub wishlist: Vec<UserBookView>,
    pub recent_events: Vec<TimelineEventView>,
    pub stats: StatsView,
    pub stat_cards: Vec<StatCard>,
}

#[derive(Template)]
#[template(path = "pages/data.html")]
pub struct DataTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub is_impersonating: bool,
    pub impersonated_username: String,
    pub active_type: String,
    pub tabs: Vec<Tab>,
    pub tab_signal: &'static str,
    pub tab_signal_js: &'static str,
    pub tab_base_url: &'static str,
    pub tab_fetch_target: &'static str,
    pub tab_fetch_mode: &'static str,
    pub content: String,
    pub search_value: String,
}

pub struct Tab {
    pub key: &'static str,
    pub label: &'static str,
}

#[derive(Template)]
#[template(path = "pages/add.html")]
pub struct AddTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub is_impersonating: bool,
    pub impersonated_username: String,
    pub active_type: String,
    pub tabs: Vec<Tab>,
    pub tab_signal: &'static str,
    pub tab_signal_js: &'static str,
    pub tab_base_url: &'static str,
    pub tab_fetch_target: &'static str,
    pub tab_fetch_mode: &'static str,
    pub genre_options: Vec<GenreOptionView>,
    pub author_options: Vec<AuthorOptionView>,
    pub book_options: Vec<BookOptionView>,
    pub selected_book_id: String,
    pub selected_status: String,
    pub today: String,
}

pub struct YearTab {
    pub key: String,
    pub label: String,
}

#[derive(Template)]
#[template(path = "pages/stats.html")]
pub struct StatsPageTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub base_url: &'static str,
    pub is_impersonating: bool,
    pub impersonated_username: String,
    pub cache_age: String,
    pub content: String,
    pub year_tabs: Vec<YearTab>,
    pub active_year: String,
}

#[derive(Template)]
#[template(path = "partials/stats_content.html")]
pub struct StatsContentTemplate {
    pub book_summary: BookSummaryStats,
    pub reading: ReadingStats,
    pub has_data: bool,
    pub is_year_view: bool,
    /// Pre-formatted pipe-separated data for the activity bar chart component.
    pub activity_chart_data: String,
    /// Pages read all-time, formatted with thousand separators.
    pub pages_formatted: String,
    /// Average rating formatted to one decimal place.
    pub avg_rating_formatted: String,
    /// Average days to finish, rounded to nearest integer.
    pub avg_days_formatted: String,
    /// Pre-formatted pipe-separated data for the rating donut chart.
    pub rating_chart_data: String,
}

#[derive(Template)]
#[template(path = "pages/author.html")]
pub struct AuthorDetailTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub base_url: &'static str,
    pub is_impersonating: bool,
    pub impersonated_username: String,
    pub author: AuthorDetailView,
    pub image_url: Option<String>,
    pub edit_url: String,
    pub library_books: Vec<AuthorBookCardView>,
}

#[derive(Template)]
#[template(path = "pages/book.html")]
pub struct BookDetailTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub base_url: &'static str,
    pub is_impersonating: bool,
    pub impersonated_username: String,
    pub book: BookDetailView,
    pub image_url: Option<String>,
    pub edit_url: String,
    pub library_info: Option<BookLibraryInfo>,
    pub readings: Vec<BookReadingCardView>,
    pub active_reading_id: Option<String>,
}

#[derive(Template)]
#[template(path = "pages/reading.html")]
pub struct ReadingDetailTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub base_url: &'static str,
    pub is_impersonating: bool,
    pub impersonated_username: String,
    pub reading: ReadingDetailView,
    pub edit_url: String,
}

#[derive(Template)]
#[template(path = "pages/genre.html")]
pub struct GenreDetailTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub base_url: &'static str,
    pub is_impersonating: bool,
    pub impersonated_username: String,
    pub genre: GenreDetailView,
    pub edit_url: String,
    pub library_books: Vec<AuthorBookCardView>,
}

// ── Edit page templates ──

#[derive(Template)]
#[template(path = "pages/edit_author.html")]
pub struct AuthorEditTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub is_impersonating: bool,
    pub impersonated_username: String,
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
}

#[derive(Template)]
#[template(path = "pages/edit_genre.html")]
pub struct GenreEditTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub is_impersonating: bool,
    pub impersonated_username: String,
    pub id: String,
    pub name: String,
}

#[derive(Template)]
#[template(path = "pages/edit_book.html")]
pub struct BookEditTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub is_impersonating: bool,
    pub impersonated_username: String,
    pub id: String,
    pub title: String,
    pub isbn: String,
    pub description: String,
    pub page_count: String,
    pub year_published: String,
    pub publisher: String,
    pub language: String,
    pub primary_genre_id: String,
    pub secondary_genre_id: String,
    pub genre_options: Vec<GenreOptionView>,
    pub author_options: Vec<AuthorOptionView>,
    pub author_id: String,
    pub image_url: Option<String>,
}

#[derive(Template)]
#[template(path = "pages/start_book.html")]
pub struct StartBookTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub is_impersonating: bool,
    pub impersonated_username: String,
    pub book_id: String,
    pub book_label: String,
    pub author_names: String,
    pub page_count_label: String,
    pub thumbnail_url: Option<String>,
    pub started_at: String,
    pub book_club: bool,
}

#[derive(Template)]
#[template(path = "pages/start_reading.html")]
pub struct StartReadingTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub is_impersonating: bool,
    pub impersonated_username: String,
    pub id: String,
    pub book_id: String,
    pub book_label: String,
    pub author_names: String,
    pub page_count_label: String,
    pub thumbnail_url: Option<String>,
    pub status: String,
    pub format: String,
    pub started_at: String,
}

#[derive(Template)]
#[template(path = "pages/finish_reading.html")]
pub struct FinishReadingTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub is_impersonating: bool,
    pub impersonated_username: String,
    pub id: String,
    pub book_id: String,
    pub book_label: String,
    pub author_names: String,
    pub page_count_label: String,
    pub thumbnail_url: Option<String>,
    pub status: String,
    pub format: String,
    pub started_at: String,
    pub finished_at: String,
    pub rating: String,
    pub quick_reviews: String,
}

#[derive(Template)]
#[template(path = "pages/edit_reading.html")]
pub struct ReadingEditTemplate {
    pub nav_active: &'static str,
    pub is_authenticated: bool,
    pub version_info: &'static crate::VersionInfo,
    pub is_impersonating: bool,
    pub impersonated_username: String,
    pub id: String,
    pub book_id: String,
    pub book_label: String,
    pub status: String,
    pub format: String,
    pub started_at: String,
    pub finished_at: String,
    pub rating: String,
    pub quick_reviews: String,
    pub book_club: bool,
    pub book_options: Vec<BookOptionView>,
}

#[derive(Template)]
#[template(path = "partials/image_upload.html")]
pub struct ImageUploadTemplate<'a> {
    pub entity_type: &'a str,
    pub entity_id: i64,
    pub image_url: Option<&'a str>,
    pub is_authenticated: bool,
}

pub fn render_template<T: Template>(template: T) -> Result<String, askama::Error> {
    template.render()
}
