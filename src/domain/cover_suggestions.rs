use chrono::{DateTime, Utc};

pub struct CoverSuggestion {
    pub id: String,
    pub image_data: Vec<u8>,
    pub thumbnail_data: Vec<u8>,
    pub content_type: String,
    pub source_url: String,
    pub created_at: DateTime<Utc>,
}
