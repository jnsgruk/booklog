use crate::domain::books::readings::{QuickReview, ReadingWithBook};
use crate::domain::formatting::{format_pages, format_rating};

use super::{or_em_dash, reading_path};

pub struct QuickReviewView {
    pub label: String,
    pub pill_class: &'static str,
    pub form_value: String,
}

impl From<QuickReview> for QuickReviewView {
    fn from(review: QuickReview) -> Self {
        let pill_class = if review.is_positive() {
            "pill pill-success"
        } else if review.is_neutral() {
            "pill pill-info"
        } else {
            "pill pill-warning"
        };
        Self {
            label: review.label().to_string(),
            pill_class,
            form_value: review.form_value().to_string(),
        }
    }
}

pub struct ReadingView {
    pub id: String,
    pub detail_path: String,
    pub book_title: String,
    pub book_id: String,
    pub author_names: String,
    pub status: String,
    pub status_label: String,
    pub rating: String,
    pub started_date: String,
    pub finished_date: String,
    pub created_date: String,
    pub created_time: String,
    pub created_at_sort_key: i64,
    pub relative_date_label: String,
    pub page_count_label: String,
    pub year_label: String,
    pub genres_label: String,
    pub thumbnail_url: Option<String>,
}

impl ReadingView {
    pub fn from_domain(rwb: ReadingWithBook) -> Self {
        let id = rwb.reading.id;
        let relative_date_label = super::relative_date(rwb.reading.created_at);
        let page_count_label = rwb.page_count.map(format_pages).unwrap_or_default();
        let year_label = rwb
            .year_published
            .map(|y| y.to_string())
            .unwrap_or_default();
        let genres_label = [&rwb.primary_genre, &rwb.secondary_genre]
            .iter()
            .filter_map(|g| g.as_deref())
            .collect::<Vec<_>>()
            .join(", ");
        Self {
            detail_path: reading_path(id),
            id: id.to_string(),
            book_title: rwb.book_title,
            book_id: rwb.reading.book_id.to_string(),
            author_names: if rwb.author_names.is_empty() {
                "Unknown".to_string()
            } else {
                rwb.author_names
            },
            status: rwb.reading.status.as_str().to_string(),
            status_label: rwb.reading.status.display_label().to_string(),
            rating: or_em_dash(rwb.reading.rating.map(format_rating)),
            started_date: or_em_dash(rwb.reading.started_at),
            finished_date: or_em_dash(rwb.reading.finished_at),
            created_date: rwb.reading.created_at.format("%Y-%m-%d").to_string(),
            created_time: rwb.reading.created_at.format("%H:%M").to_string(),
            created_at_sort_key: rwb.reading.created_at.timestamp(),
            relative_date_label,
            page_count_label,
            year_label,
            genres_label,
            thumbnail_url: None,
        }
    }
}

pub struct ReadingDetailView {
    pub id: String,
    pub book_title: String,
    pub book_id: String,
    pub author_names: String,
    pub status_label: String,
    pub format_label: String,
    pub rating: String,
    pub quick_reviews: Vec<QuickReviewView>,
    pub started_date: String,
    pub finished_date: String,
    pub created_date: String,
    pub created_time: String,
}

impl ReadingDetailView {
    pub fn from_domain(rwb: ReadingWithBook) -> Self {
        Self {
            id: rwb.reading.id.to_string(),
            book_title: rwb.book_title,
            book_id: rwb.reading.book_id.to_string(),
            author_names: if rwb.author_names.is_empty() {
                "Unknown".to_string()
            } else {
                rwb.author_names
            },
            status_label: rwb.reading.status.display_label().to_string(),
            format_label: or_em_dash(rwb.reading.format.map(|f| f.display_label())),
            rating: or_em_dash(rwb.reading.rating.map(format_rating)),
            quick_reviews: rwb
                .reading
                .quick_reviews
                .iter()
                .copied()
                .map(QuickReviewView::from)
                .collect(),
            started_date: or_em_dash(rwb.reading.started_at),
            finished_date: or_em_dash(rwb.reading.finished_at),
            created_date: rwb.reading.created_at.format("%Y-%m-%d").to_string(),
            created_time: rwb.reading.created_at.format("%H:%M").to_string(),
        }
    }
}
