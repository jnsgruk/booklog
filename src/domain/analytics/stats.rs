use serde::{Deserialize, Serialize};

/// Summary statistics for books: genres, authors, page counts, publication years.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BookSummaryStats {
    pub total_books: u64,
    pub total_authors: u64,
    pub unique_genres: u64,
    pub top_genre: Option<String>,
    pub top_author: Option<String>,
    /// Author with the highest total sum of star ratings.
    #[serde(default)]
    pub most_rated_author: Option<String>,
    /// Genre with the highest total sum of star ratings.
    #[serde(default)]
    pub most_rated_genre: Option<String>,
    pub genre_counts: Vec<(String, u64)>,
    pub max_genre_count: u64,
    /// Page count distribution buckets (e.g. "< 200", "200 – 350", "350 – 500", "500+").
    #[serde(default)]
    pub page_count_distribution: Vec<(String, u64)>,
    /// Publication year by decade (e.g. "1990s", "2000s").
    #[serde(default)]
    pub year_published_distribution: Vec<(String, u64)>,
    #[serde(default)]
    pub max_year_published_count: u64,
    /// Top authors ranked by completed readings.
    #[serde(default)]
    pub top_authors: Vec<(String, u64)>,
    #[serde(default)]
    pub max_top_author_count: u64,
    /// Longest book by page count: (title, `page_count`).
    #[serde(default)]
    pub longest_book: Option<(String, i32)>,
    /// Shortest book by page count: (title, `page_count`).
    #[serde(default)]
    pub shortest_book: Option<(String, i32)>,
}

/// Reading activity totals and distributions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReadingStats {
    pub books_last_30_days: u64,
    pub books_all_time: u64,
    pub pages_last_30_days: i64,
    pub pages_all_time: i64,
    pub books_in_progress: u64,
    #[serde(default)]
    pub books_on_shelf: u64,
    pub books_on_wishlist: u64,
    pub average_rating: Option<f64>,
    /// Count of abandoned readings.
    #[serde(default)]
    pub books_abandoned: u64,
    /// Average days from `started_at` to `finished_at` for completed readings.
    #[serde(default)]
    pub average_days_to_finish: Option<f64>,
    /// Rating distribution: (`rating_value`, count) for each half-star value.
    #[serde(default)]
    pub rating_distribution: Vec<(f64, u64)>,
    #[serde(default)]
    pub max_rating_count: u64,
    /// Books finished per month in the current year.
    #[serde(default)]
    pub monthly_books: Vec<(String, u64)>,
    /// Pages read per month in the current year.
    #[serde(default)]
    pub monthly_pages: Vec<(String, i64)>,
    #[serde(default)]
    pub max_monthly_books: u64,
    #[serde(default)]
    pub max_monthly_pages: i64,
    /// Books finished per year (all-time view).
    #[serde(default)]
    pub yearly_books: Vec<(String, u64)>,
    /// Pages read per year (all-time view).
    #[serde(default)]
    pub yearly_pages: Vec<(String, i64)>,
    #[serde(default)]
    pub max_yearly_books: u64,
    #[serde(default)]
    pub max_yearly_pages: i64,
    /// Reading pace distribution: Slow / Medium / Fast based on pages per day.
    #[serde(default)]
    pub pace_distribution: Vec<(String, u64)>,
    /// Format distribution: Physical / eReader / Audiobook.
    #[serde(default)]
    pub format_counts: Vec<(String, u64)>,
}

/// Pre-computed snapshot of all statistics, stored as JSON in the cache table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedStats {
    pub book_summary: BookSummaryStats,
    pub reading: ReadingStats,
    pub computed_at: String,
}
