use crate::domain::analytics::timeline::{TimelineEvent, TimelineEventDetail};
use crate::domain::books::quick_reviews::QuickReview;

use super::{author_path, book_path, genre_path, reading_path, relative_date};

/// Returns `true` if the value is empty or just an em-dash placeholder.
fn is_blank(value: &str) -> bool {
    value.is_empty() || value == crate::domain::formatting::EM_DASH
}

#[derive(Clone)]
pub struct TimelineEventDetailView {
    pub label: String,
    pub value: String,
    pub link: Option<String>,
}

/// Reading data for timeline events.
#[derive(Clone)]
pub struct TimelineReadingDataView {
    pub book_id: i64,
    pub rating: Option<i32>,
    pub status: String,
}

#[derive(Clone)]
pub struct QuickNoteView {
    pub label: String,
    pub pill_class: &'static str,
}

#[derive(Clone)]
pub struct TimelineEventView {
    pub id: String,
    pub entity_type: String,
    pub kind_label: &'static str,
    pub date_label: String,
    pub relative_date_label: String,
    pub time_label: Option<String>,
    pub iso_timestamp: String,
    pub title: String,
    pub link: String,
    pub external_link: Option<String>,
    pub details: Vec<TimelineEventDetailView>,
    pub subtitle: Option<String>,
    pub genres: Option<Vec<String>>,
    pub quick_notes: Option<Vec<QuickNoteView>>,
    pub reading_data: Option<TimelineReadingDataView>,
}

pub struct TimelineMonthView {
    pub anchor: String,
    pub heading: String,
    pub events: Vec<TimelineEventView>,
}

impl TimelineEventView {
    pub fn from_domain(event: TimelineEvent) -> Self {
        let TimelineEvent {
            id,
            entity_type,
            entity_id,
            action,
            occurred_at,
            title,
            details,
            genres,
            reading_data,
        } = event;

        let kind_label = match (entity_type.as_str(), action.as_str()) {
            ("author", "added") => "Author Added",
            ("book", "added") => "Book Added",
            ("book", "shelved") => "Shelved",
            ("reading", "want to read") => "Want to Read",
            ("reading", "started") => "Started",
            ("reading", "finished") => "Finished",
            ("reading", "abandoned") => "Abandoned",
            ("genre", "added") => "Genre Added",
            _ => "Event",
        };

        let link = match entity_type.as_str() {
            "author" => author_path(entity_id),
            "book" => book_path(entity_id),
            "reading" => reading_path(entity_id),
            "genre" => genre_path(entity_id),
            _ => String::from("#"),
        };

        let (mapped_details, external_link, quick_notes) = Self::map_details(details);
        let subtitle = Self::build_subtitle(entity_type.as_str(), &mapped_details);

        // For books, genres contains genre tags
        let genres = if entity_type == "book" && !genres.is_empty() {
            Some(genres)
        } else {
            None
        };

        let reading_data_view = reading_data.map(|rd| TimelineReadingDataView {
            book_id: rd.book_id,
            rating: rd.rating,
            status: rd.status,
        });

        Self {
            id: id.to_string(),
            entity_type,
            kind_label,
            date_label: occurred_at.format("%b %d, %y").to_string(),
            relative_date_label: relative_date(occurred_at),
            time_label: Some(occurred_at.format("%H:%M UTC").to_string()),
            iso_timestamp: occurred_at.to_rfc3339(),
            title,
            link,
            external_link,
            details: mapped_details,
            subtitle,
            genres,
            quick_notes,
            reading_data: reading_data_view,
        }
    }

    fn build_subtitle(entity_type: &str, details: &[TimelineEventDetailView]) -> Option<String> {
        let find_value = |label: &str| {
            details
                .iter()
                .find(|d| d.label.eq_ignore_ascii_case(label))
                .map(|d| d.value.trim())
                .filter(|v| !is_blank(v))
        };

        let picks: &[&str] = match entity_type {
            "book" => &["Author"],
            "reading" => &["Author", "Rating"],
            "author" => &["Nationality"],
            _ => &[],
        };

        let parts: Vec<&str> = if picks.is_empty() {
            details
                .iter()
                .take(3)
                .map(|d| d.value.trim())
                .filter(|v| !is_blank(v))
                .collect()
        } else {
            picks.iter().filter_map(|l| find_value(l)).collect()
        };

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" \u{00b7} "))
        }
    }

    fn map_details(
        details: Vec<TimelineEventDetail>,
    ) -> (
        Vec<TimelineEventDetailView>,
        Option<String>,
        Option<Vec<QuickNoteView>>,
    ) {
        let mut mapped = Vec::new();
        let mut external_link = None;
        let mut quick_notes = Vec::new();

        for detail in details {
            let label_lower = detail.label.to_ascii_lowercase();
            match label_lower.as_str() {
                "website" => {
                    let trimmed = detail.value.trim();
                    if !is_blank(trimmed) {
                        external_link = Some(trimmed.to_string());
                    }
                }
                "notes" => {
                    for label in detail.value.split(", ") {
                        let pill_class =
                            QuickReview::from_str_value(label).map_or("pill-muted", |qr| match qr
                                .sentiment()
                            {
                                crate::domain::books::quick_reviews::Sentiment::Positive => {
                                    "pill-success"
                                }
                                crate::domain::books::quick_reviews::Sentiment::Neutral => {
                                    "pill-muted"
                                }
                                crate::domain::books::quick_reviews::Sentiment::Negative => {
                                    "pill-warning"
                                }
                            });
                        quick_notes.push(QuickNoteView {
                            label: label.to_string(),
                            pill_class,
                        });
                    }
                }
                _ => {
                    mapped.push(TimelineEventDetailView {
                        label: detail.label,
                        value: detail.value,
                        link: None,
                    });
                }
            }
        }

        let quick_notes = if quick_notes.is_empty() {
            None
        } else {
            Some(quick_notes)
        };

        (mapped, external_link, quick_notes)
    }
}
