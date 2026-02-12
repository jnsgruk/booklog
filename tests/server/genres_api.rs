use crate::helpers::{create_genre_with_name, spawn_app_with_auth};
use crate::test_macros::define_crud_tests;
use booklog::domain::genres::{Genre, NewGenre, UpdateGenre};

define_crud_tests!(
    entity: genre,
    path: "/genres",
    list_type: Genre,
    malformed_json: r#"{"name": "Test", "created_at": }"#,
    missing_fields: r#"{}"#
);

#[tokio::test]
async fn creating_a_genre_returns_a_201_for_valid_data() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let new_genre = NewGenre {
        name: "Science Fiction".to_string(),
        created_at: None,
    };

    let response = client
        .post(app.api_url("/genres"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_genre)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 201);

    let genre: Genre = response.json().await.expect("Failed to parse response");
    assert_eq!(genre.name, "Science Fiction");
}

#[tokio::test]
async fn creating_a_genre_persists_the_data() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let new_genre = NewGenre {
        name: "Persistent Genre".to_string(),
        created_at: None,
    };

    let create_response = client
        .post(app.api_url("/genres"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_genre)
        .send()
        .await
        .expect("Failed to execute request");

    let genre: Genre = create_response
        .json()
        .await
        .expect("Failed to parse response");

    let get_response = client
        .get(app.api_url(&format!("/genres/{}", genre.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(get_response.status(), 200);

    let fetched_genre: Genre = get_response.json().await.expect("Failed to parse response");
    assert_eq!(fetched_genre.name, "Persistent Genre");
}

#[tokio::test]
async fn getting_a_genre_returns_a_200_for_valid_id() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let genre = create_genre_with_name(&app, "Fetchable Genre").await;

    let response = client
        .get(app.api_url(&format!("/genres/{}", genre.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let fetched: Genre = response.json().await.expect("Failed to parse response");
    assert_eq!(fetched.id, genre.id);
    assert_eq!(fetched.name, "Fetchable Genre");
}

#[tokio::test]
async fn listing_genres_returns_a_200_with_multiple_genres() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let genre1 = NewGenre {
        name: "First Genre".to_string(),
        created_at: None,
    };

    let genre2 = NewGenre {
        name: "Second Genre".to_string(),
        created_at: None,
    };

    client
        .post(app.api_url("/genres"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&genre1)
        .send()
        .await
        .expect("Failed to create first genre");

    client
        .post(app.api_url("/genres"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&genre2)
        .send()
        .await
        .expect("Failed to create second genre");

    let response = client
        .get(app.api_url("/genres"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let genres: Vec<Genre> = response.json().await.expect("Failed to parse response");
    assert_eq!(genres.len(), 2);
}

#[tokio::test]
async fn updating_a_genre_returns_a_200_for_valid_data() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let genre = create_genre_with_name(&app, "Original Genre").await;

    let update = UpdateGenre {
        name: Some("Updated Genre".to_string()),
        created_at: None,
    };

    let response = client
        .put(app.api_url(&format!("/genres/{}", genre.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&update)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let updated_genre: Genre = response.json().await.expect("Failed to parse response");
    assert_eq!(updated_genre.name, "Updated Genre");
}

#[tokio::test]
async fn updating_a_genre_with_no_changes_returns_a_400() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let genre = create_genre_with_name(&app, "Test Genre").await;

    let update = UpdateGenre {
        name: None,
        created_at: None,
    };

    let response = client
        .put(app.api_url(&format!("/genres/{}", genre.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&update)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn updating_a_nonexistent_genre_returns_a_404() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let update = UpdateGenre {
        name: Some("New Name".to_string()),
        created_at: None,
    };

    let response = client
        .put(app.api_url("/genres/999999"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&update)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn deleting_a_genre_returns_a_204_for_valid_id() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let genre = create_genre_with_name(&app, "To Be Deleted").await;

    let response = client
        .delete(app.api_url(&format!("/genres/{}", genre.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 204);

    let get_response = client
        .get(app.api_url(&format!("/genres/{}", genre.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(get_response.status(), 404);
}

#[tokio::test]
async fn creating_a_genre_with_duplicate_name_returns_409() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let new_genre = NewGenre {
        name: "Duplicate Genre".to_string(),
        created_at: None,
    };

    // First create succeeds
    let response = client
        .post(app.api_url("/genres"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_genre)
        .send()
        .await
        .expect("Failed to create first genre");

    assert_eq!(response.status(), 201);

    // Second create with same name should fail
    let response = client
        .post(app.api_url("/genres"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_genre)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 409);
}
