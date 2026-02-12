use std::time::Duration;

use tracing::warn;
use uuid::Uuid;

use crate::domain::cover_suggestions::CoverSuggestion;
use crate::infrastructure::image_processing::process_image_bytes;

const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_SUGGESTIONS: usize = 5;

/// Download cover images from the given URLs, process them into thumbnails,
/// and return `CoverSuggestion` values ready for storage.
///
/// Skips URLs that fail to download or process. Returns at most `MAX_SUGGESTIONS` results.
pub async fn fetch_cover_images(client: &reqwest::Client, urls: &[String]) -> Vec<CoverSuggestion> {
    let urls: Vec<&String> = urls.iter().take(MAX_SUGGESTIONS).collect();

    let futures: Vec<_> = urls
        .iter()
        .map(|url| download_and_process(client, url))
        .collect();

    let results = futures::future::join_all(futures).await;

    results.into_iter().flatten().collect()
}

async fn download_and_process(client: &reqwest::Client, url: &str) -> Option<CoverSuggestion> {
    let response = match client.get(url).timeout(DOWNLOAD_TIMEOUT).send().await {
        Ok(r) => r,
        Err(err) => {
            warn!(url, error = %err, "failed to download cover image");
            return None;
        }
    };

    if !response.status().is_success() {
        warn!(url, status = %response.status(), "cover image download returned non-success");
        return None;
    }

    // Validate content type is an image
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !content_type.starts_with("image/") {
        warn!(url, content_type, "cover image URL did not return an image");
        return None;
    }

    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(err) => {
            warn!(url, error = %err, "failed to read cover image bytes");
            return None;
        }
    };

    if bytes.is_empty() {
        warn!(url, "cover image download returned empty body");
        return None;
    }

    let url_owned = url.to_string();
    let processed = match tokio::task::spawn_blocking(move || process_image_bytes(&bytes)).await {
        Ok(Ok(p)) => p,
        Ok(Err(err)) => {
            warn!(url = url_owned, error = %err, "failed to process cover image");
            return None;
        }
        Err(err) => {
            warn!(url = url_owned, error = %err, "cover image processing task panicked");
            return None;
        }
    };

    Some(CoverSuggestion {
        id: Uuid::new_v4().to_string(),
        image_data: processed.image_data,
        thumbnail_data: processed.thumbnail_data,
        content_type: processed.content_type,
        source_url: url_owned,
        created_at: chrono::Utc::now(),
    })
}
