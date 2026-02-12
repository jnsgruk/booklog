use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::application::errors::AppError;

pub const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const USER_AGENT: &str = "Booklog/1.0";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

const AUTHOR_PROMPT: &str = r#"Extract author information from this input. Use web search to look up any details you cannot determine from the input alone. Return a JSON object with these fields (only include fields you can identify with confidence):
- "name": the author's name

Return ONLY the JSON object, no other text."#;

const COVER_PROMPT: &str = r#"Find book cover images for the book described below. Search for cover images on OpenLibrary (e.g. https://covers.openlibrary.org/b/isbn/{isbn}-L.jpg), Google Books, Amazon, and other web properties. Return a JSON object with this field:
- "cover_image_urls": an array of up to 5 URLs to book cover images. Include only direct image URLs that point to actual cover art. Do *not* include images that contain the text 'image not available'. 

Return ONLY the JSON object, no other text."#;

fn book_prompt(available_genres: &[String]) -> String {
    let mut prompt = String::from(
        r#"Extract book information from this input. Use web search to look up any details you cannot determine from the input alone (e.g. author, ISBN, page count, publisher, genres). Return a JSON object with these fields (only include fields you can identify with confidence):
- "title": the book's title
- "author_name": the name of the primary author
- "isbn": the ISBN (10 or 13 digit)
- "description": a brief description or summary of the book
- "page_count": the number of pages
- "year_published": the year the book was first published
- "publisher": the publisher's name
- "language": the language the book is written in
- "primary_genre": the single best-matching genre for this book
- "secondary_genre": an optional secondary genre for this book
- "cover_image_urls": an array of up to 5 URLs to book cover images. Search for cover images on OpenLibrary (e.g. https://covers.openlibrary.org/b/isbn/{isbn}-L.jpg), Google Books, Amazon, and other web properties. Include only direct image URLs that point to actual cover art.

Return ONLY the JSON object, no other text."#,
    );

    if !available_genres.is_empty() {
        use std::fmt::Write;
        let _ = write!(
            prompt,
            "\n\nHere are the available genres to choose from (prefer these, but suggest new ones if none fit well): {}",
            available_genres.join(", ")
        );
    }

    prompt
}

// --- Public types ---

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub cost: f64,
}

#[derive(Debug, Deserialize)]
pub struct ExtractionInput {
    pub image: Option<String>,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedAuthor {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedBook {
    pub title: Option<String>,
    pub author_name: Option<String>,
    pub isbn: Option<String>,
    pub description: Option<String>,
    pub page_count: Option<i32>,
    pub year_published: Option<i32>,
    pub publisher: Option<String>,
    pub language: Option<String>,
    pub primary_genre: Option<String>,
    pub secondary_genre: Option<String>,
    pub cover_image_urls: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtractedCovers {
    pub cover_image_urls: Option<Vec<String>>,
}

// --- Public functions ---

pub async fn extract_author(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    model: &str,
    input: &ExtractionInput,
) -> Result<(ExtractedAuthor, Option<Usage>), AppError> {
    let (content, usage) =
        call_openrouter(client, url, api_key, model, AUTHOR_PROMPT, input).await?;
    let json = extract_json(&content);

    let extracted = serde_json::from_str(json).map_err(|e| {
        AppError::unexpected(format!("Failed to parse AI response as author data: {e}"))
    })?;
    Ok((extracted, usage))
}

pub async fn extract_book(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    model: &str,
    input: &ExtractionInput,
    available_genres: &[String],
) -> Result<(ExtractedBook, Option<Usage>), AppError> {
    let prompt = book_prompt(available_genres);
    let (content, usage) = call_openrouter(client, url, api_key, model, &prompt, input).await?;
    let json = extract_json(&content);

    let extracted = serde_json::from_str(json).map_err(|e| {
        AppError::unexpected(format!("Failed to parse AI response as book data: {e}"))
    })?;
    Ok((extracted, usage))
}

pub async fn fetch_cover_urls(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    model: &str,
    title: &str,
    author: &str,
    isbn: Option<&str>,
) -> Result<(ExtractedCovers, Option<Usage>), AppError> {
    use std::fmt::Write;
    let mut description = format!("\"{title}\" by {author}");
    if let Some(isbn) = isbn.filter(|s| !s.is_empty()) {
        let _ = write!(description, " (ISBN: {isbn})");
    }

    let input = ExtractionInput {
        image: None,
        prompt: Some(description),
    };
    let (content, usage) =
        call_openrouter(client, url, api_key, model, COVER_PROMPT, &input).await?;
    let json = extract_json(&content);

    let extracted = serde_json::from_str(json).map_err(|e| {
        AppError::unexpected(format!("Failed to parse AI response as cover data: {e}"))
    })?;
    Ok((extracted, usage))
}

// --- Internal helpers ---

async fn call_openrouter(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    input: &ExtractionInput,
) -> Result<(String, Option<Usage>), AppError> {
    let has_image = input.image.as_ref().is_some_and(|s| !s.trim().is_empty());
    let has_prompt = input.prompt.as_ref().is_some_and(|s| !s.trim().is_empty());

    if !has_image && !has_prompt {
        return Err(AppError::validation(
            "Provide either an image or a text prompt",
        ));
    }

    let mut content_parts = vec![ContentPart::Text {
        text: system_prompt.to_string(),
    }];

    if let Some(image) = &input.image
        && !image.trim().is_empty()
    {
        content_parts.push(ContentPart::ImageUrl {
            image_url: ImageUrlDetail { url: image.clone() },
        });
    }

    if let Some(prompt) = &input.prompt
        && !prompt.trim().is_empty()
    {
        content_parts.push(ContentPart::Text {
            text: prompt.clone(),
        });
    }

    let request_body = ChatRequest {
        model: model.to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: content_parts,
        }],
    };

    let response = client
        .post(url)
        .header("User-Agent", USER_AGENT)
        .header("Authorization", format!("Bearer {api_key}"))
        .timeout(REQUEST_TIMEOUT)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| AppError::unexpected(format!("OpenRouter request failed: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "(unreadable body)".to_string());
        return Err(AppError::unexpected(format!(
            "OpenRouter returned status {status}: {body}"
        )));
    }

    let body = response.text().await.map_err(|e| {
        AppError::unexpected(format!("Failed to read OpenRouter response body: {e}"))
    })?;

    let chat_response: ChatResponse = serde_json::from_str(&body)
        .map_err(|e| AppError::unexpected(format!("Failed to parse OpenRouter response: {e}")))?;

    let content = chat_response
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_default();

    if content.trim().is_empty() {
        return Err(AppError::unexpected(
            "OpenRouter returned an empty response".to_string(),
        ));
    }

    Ok((content, chat_response.usage))
}

/// Extract a JSON object from a model response that may contain markdown
/// fences (```json ... ```) or surrounding prose.
fn extract_json(raw: &str) -> &str {
    let trimmed = raw.trim();

    // Strip ```json ... ``` or ``` ... ``` fences
    if let Some(after) = trimmed.strip_prefix("```json")
        && let Some(inner) = after.strip_suffix("```")
    {
        return inner.trim();
    }
    if let Some(after) = trimmed.strip_prefix("```")
        && let Some(inner) = after.strip_suffix("```")
    {
        return inner.trim();
    }

    // Find the first '{' and last '}' to extract the JSON object
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}'))
        && start < end
    {
        return &trimmed[start..=end];
    }

    trimmed
}

// --- OpenRouter API types ---

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: Vec<ContentPart>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlDetail },
}

#[derive(Debug, Serialize)]
struct ImageUrlDetail {
    url: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_chat_response() {
        let json = r#"{
            "id": "gen-abc123",
            "model": "openrouter/free",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "{\"name\": \"Ursula K. Le Guin\"}"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 194,
                "completion_tokens": 42,
                "total_tokens": 236,
                "cost": 0.0012
            }
        }"#;

        let response: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices.len(), 1);

        let content = &response.choices[0].message.content;
        let author: ExtractedAuthor = serde_json::from_str(content).unwrap();
        assert_eq!(author.name.as_deref(), Some("Ursula K. Le Guin"));

        let usage = response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 194);
        assert_eq!(usage.completion_tokens, 42);
        assert_eq!(usage.total_tokens, 236);
        assert!((usage.cost - 0.0012).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_chat_response_without_usage() {
        let json = r#"{
            "id": "gen-abc123",
            "model": "openrouter/free",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "{\"name\": \"Ursula K. Le Guin\"}"
                    },
                    "finish_reason": "stop"
                }
            ]
        }"#;

        let response: ChatResponse = serde_json::from_str(json).unwrap();
        assert!(response.usage.is_none());
    }

    #[test]
    fn parse_book_extraction() {
        let json = r#"{
            "title": "The Left Hand of Darkness",
            "author_name": "Ursula K. Le Guin",
            "isbn": "9780441478125",
            "description": "A groundbreaking science fiction novel exploring gender and politics on a distant planet.",
            "page_count": 304,
            "year_published": 1969,
            "publisher": "Ace Books",
            "language": "English",
            "primary_genre": "Science Fiction",
            "secondary_genre": "Literary Fiction"
        }"#;

        let book: ExtractedBook = serde_json::from_str(json).unwrap();
        assert_eq!(book.title.as_deref(), Some("The Left Hand of Darkness"));
        assert_eq!(book.author_name.as_deref(), Some("Ursula K. Le Guin"));
        assert_eq!(book.isbn.as_deref(), Some("9780441478125"));
        assert_eq!(book.page_count, Some(304));
        assert_eq!(book.year_published, Some(1969));
        assert_eq!(book.primary_genre.as_deref(), Some("Science Fiction"));
        assert_eq!(book.secondary_genre.as_deref(), Some("Literary Fiction"));
    }

    #[test]
    fn parse_partial_book_extraction() {
        let json = r#"{"title": "1984", "author_name": "George Orwell"}"#;

        let book: ExtractedBook = serde_json::from_str(json).unwrap();
        assert_eq!(book.title.as_deref(), Some("1984"));
        assert_eq!(book.author_name.as_deref(), Some("George Orwell"));
        assert!(book.isbn.is_none());
        assert!(book.page_count.is_none());
        assert!(book.primary_genre.is_none());
        assert!(book.secondary_genre.is_none());
    }

    #[test]
    fn serialize_chat_request_with_image() {
        let request = ChatRequest {
            model: "test-model".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: vec![
                    ContentPart::Text {
                        text: "Extract info".to_string(),
                    },
                    ContentPart::ImageUrl {
                        image_url: ImageUrlDetail {
                            url: "data:image/jpeg;base64,/9j/4AAQ".to_string(),
                        },
                    },
                ],
            }],
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "test-model");
        assert_eq!(json["messages"][0]["content"][0]["type"], "text");
        assert_eq!(json["messages"][0]["content"][1]["type"], "image_url");
    }

    #[test]
    fn extract_json_from_plain_json() {
        let raw = r#"{"name": "Ursula K. Le Guin"}"#;
        assert_eq!(extract_json(raw), raw);
    }

    #[test]
    fn extract_json_from_markdown_fence() {
        let raw = "```json\n{\"name\": \"Ursula K. Le Guin\"}\n```";
        assert_eq!(extract_json(raw), r#"{"name": "Ursula K. Le Guin"}"#);
    }

    #[test]
    fn extract_json_from_prose() {
        let raw = "Here is the data:\n{\"name\": \"Ursula K. Le Guin\"}\nHope that helps!";
        assert_eq!(extract_json(raw), r#"{"name": "Ursula K. Le Guin"}"#);
    }
}
