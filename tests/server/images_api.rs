use booklog::domain::authors::Author;

use crate::helpers::{
    assert_datastar_headers, assert_html_fragment, create_default_author, spawn_app_with_auth,
};

/// Generate a minimal valid 1x1 red PNG as a base64 data URL.
fn tiny_png_data_url() -> String {
    use base64::Engine;
    use image::{ImageBuffer, Rgba};

    let img = ImageBuffer::from_pixel(1, 1, Rgba([255u8, 0, 0, 255]));
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(encoder, img.as_raw(), 1, 1, image::ColorType::Rgba8.into())
        .expect("failed to encode test PNG");

    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
    format!("data:image/png;base64,{b64}")
}

fn image_url(entity_type: &str, id: impl std::fmt::Display) -> String {
    format!("/{entity_type}/{id}/image")
}

fn thumbnail_url(entity_type: &str, id: impl std::fmt::Display) -> String {
    format!("/{entity_type}/{id}/thumbnail")
}

async fn upload_image(
    client: &reqwest::Client,
    app: &crate::helpers::TestApp,
    entity_type: &str,
    id: impl std::fmt::Display,
) -> reqwest::Response {
    client
        .put(app.api_url(&image_url(entity_type, &id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&serde_json::json!({ "image": tiny_png_data_url() }))
        .send()
        .await
        .expect("failed to upload image")
}

// ===========================================================================
// Upload & retrieval
// ===========================================================================

#[tokio::test]
async fn upload_image_returns_204() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();
    let author = create_default_author(&app).await;

    let response = upload_image(&client, &app, "author", author.id).await;
    assert_eq!(response.status(), 204);
}

#[tokio::test]
async fn get_image_returns_uploaded_data() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();
    let author = create_default_author(&app).await;

    upload_image(&client, &app, "author", author.id).await;

    let response = client
        .get(app.api_url(&image_url("author", author.id)))
        .send()
        .await
        .expect("failed to get image");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("image/jpeg")
    );
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok()),
        Some("public, max-age=604800")
    );
    let body = response.bytes().await.expect("failed to read body");
    assert!(!body.is_empty(), "image body should not be empty");
}

#[tokio::test]
async fn get_thumbnail_returns_image() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();
    let author = create_default_author(&app).await;

    upload_image(&client, &app, "author", author.id).await;

    let response = client
        .get(app.api_url(&thumbnail_url("author", author.id)))
        .send()
        .await
        .expect("failed to get thumbnail");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("image/jpeg")
    );
    let body = response.bytes().await.expect("failed to read body");
    assert!(!body.is_empty(), "thumbnail body should not be empty");
}

#[tokio::test]
async fn upload_image_upserts() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();
    let author = create_default_author(&app).await;

    // Upload twice
    upload_image(&client, &app, "author", author.id).await;
    upload_image(&client, &app, "author", author.id).await;

    // GET should still work (upsert, not duplicate)
    let response = client
        .get(app.api_url(&image_url("author", author.id)))
        .send()
        .await
        .expect("failed to get image");
    assert_eq!(response.status(), 200);
}

// ===========================================================================
// Deletion
// ===========================================================================

#[tokio::test]
async fn delete_image_returns_204() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();
    let author = create_default_author(&app).await;

    upload_image(&client, &app, "author", author.id).await;

    let response = client
        .delete(app.api_url(&image_url("author", author.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("failed to delete image");

    assert_eq!(response.status(), 204);
}

#[tokio::test]
async fn delete_image_makes_get_return_404() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();
    let author = create_default_author(&app).await;

    upload_image(&client, &app, "author", author.id).await;

    client
        .delete(app.api_url(&image_url("author", author.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("failed to delete image");

    let response = client
        .get(app.api_url(&image_url("author", author.id)))
        .send()
        .await
        .expect("failed to get image");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn deleting_entity_deletes_image() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();
    let author = create_default_author(&app).await;

    upload_image(&client, &app, "author", author.id).await;

    // Delete the author entity
    client
        .delete(app.api_url(&format!("/authors/{}", author.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("failed to delete author");

    // Image should be gone too
    let response = client
        .get(app.api_url(&image_url("author", author.id)))
        .send()
        .await
        .expect("failed to get image");

    assert_eq!(response.status(), 404);
}

// ===========================================================================
// Auth
// ===========================================================================

#[tokio::test]
async fn upload_image_requires_auth() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();
    let author = create_default_author(&app).await;

    let response = client
        .put(app.api_url(&image_url("author", author.id)))
        .json(&serde_json::json!({ "image": tiny_png_data_url() }))
        .send()
        .await
        .expect("failed to send request");

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn delete_image_requires_auth() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();
    let author = create_default_author(&app).await;

    upload_image(&client, &app, "author", author.id).await;

    let response = client
        .delete(app.api_url(&image_url("author", author.id)))
        .send()
        .await
        .expect("failed to send request");

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn get_image_does_not_require_auth() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();
    let author = create_default_author(&app).await;

    upload_image(&client, &app, "author", author.id).await;

    // GET without auth token
    let response = client
        .get(app.api_url(&image_url("author", author.id)))
        .send()
        .await
        .expect("failed to get image");

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn get_thumbnail_does_not_require_auth() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();
    let author = create_default_author(&app).await;

    upload_image(&client, &app, "author", author.id).await;

    let response = client
        .get(app.api_url(&thumbnail_url("author", author.id)))
        .send()
        .await
        .expect("failed to get thumbnail");

    assert_eq!(response.status(), 200);
}

// ===========================================================================
// Validation
// ===========================================================================

#[tokio::test]
async fn upload_image_invalid_entity_type_returns_400() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let response = client
        .put(app.api_url("/invalid/1/image"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&serde_json::json!({ "image": tiny_png_data_url() }))
        .send()
        .await
        .expect("failed to send request");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn upload_image_nonexistent_entity_returns_404() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let response = client
        .put(app.api_url("/author/999999/image"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&serde_json::json!({ "image": tiny_png_data_url() }))
        .send()
        .await
        .expect("failed to send request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn get_image_nonexistent_returns_404() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();
    let author = create_default_author(&app).await;

    let response = client
        .get(app.api_url(&image_url("author", author.id)))
        .send()
        .await
        .expect("failed to get image");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn get_image_invalid_entity_type_returns_400() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let response = client
        .get(app.api_url("/invalid/1/image"))
        .send()
        .await
        .expect("failed to get image");

    assert_eq!(response.status(), 400);
}

// ===========================================================================
// Datastar
// ===========================================================================

#[tokio::test]
async fn upload_image_with_datastar_returns_fragment() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();
    let author = create_default_author(&app).await;

    let response = client
        .put(app.api_url(&image_url("author", author.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("datastar-request", "true")
        .json(&serde_json::json!({ "image": tiny_png_data_url() }))
        .send()
        .await
        .expect("failed to upload image");

    assert_eq!(response.status(), 200);
    assert_datastar_headers(&response, "#entity-image");

    let body = response.text().await.expect("failed to read body");
    assert_html_fragment(&body);
}

#[tokio::test]
async fn delete_image_with_datastar_returns_fragment() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();
    let author = create_default_author(&app).await;

    upload_image(&client, &app, "author", author.id).await;

    let response = client
        .delete(app.api_url(&image_url("author", author.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .header("datastar-request", "true")
        .send()
        .await
        .expect("failed to delete image");

    assert_eq!(response.status(), 200);
    assert_datastar_headers(&response, "#entity-image");

    let body = response.text().await.expect("failed to read body");
    assert_html_fragment(&body);
}

// ===========================================================================
// Deferred image via create forms
// ===========================================================================

#[tokio::test]
async fn create_author_with_image_saves_image() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "name": "Image Author",
        "image": tiny_png_data_url(),
    });

    let response = client
        .post(app.api_url("/authors"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("failed to create author");

    assert_eq!(response.status(), 201);
    let author: Author = response.json().await.expect("failed to parse author");

    // Image should be retrievable
    let img_response = client
        .get(app.api_url(&image_url("author", author.id)))
        .send()
        .await
        .expect("failed to get image");

    assert_eq!(img_response.status(), 200);
}

#[tokio::test]
async fn create_author_without_image_field_works() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "name": "No Image Author",
    });

    let response = client
        .post(app.api_url("/authors"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&payload)
        .send()
        .await
        .expect("failed to create author");

    assert_eq!(response.status(), 201);
    let author: Author = response.json().await.expect("failed to parse author");

    // No image
    let img_response = client
        .get(app.api_url(&image_url("author", author.id)))
        .send()
        .await
        .expect("failed to get image");

    assert_eq!(img_response.status(), 404);
}
