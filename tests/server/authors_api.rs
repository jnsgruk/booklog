use crate::helpers::{create_author_with_name, spawn_app_with_auth};
use crate::test_macros::define_crud_tests;
use booklog::domain::authors::{Author, NewAuthor, UpdateAuthor};

define_crud_tests!(
    entity: author,
    path: "/authors",
    list_type: Author,
    malformed_json: r#"{"name": "Test", "created_at": }"#,
    missing_fields: r#"{}"#
);

#[tokio::test]
async fn creating_an_author_returns_a_201_for_valid_data() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let new_author = NewAuthor {
        name: "Ursula K. Le Guin".to_string(),
        created_at: None,
    };

    let response = client
        .post(app.api_url("/authors"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_author)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 201);

    let author: Author = response.json().await.expect("Failed to parse response");
    assert_eq!(author.name, "Ursula K. Le Guin");
}

#[tokio::test]
async fn creating_an_author_persists_the_data() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let new_author = NewAuthor {
        name: "Persistent Author".to_string(),
        created_at: None,
    };

    let response = client
        .post(app.api_url("/authors"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_author)
        .send()
        .await
        .expect("Failed to execute request");

    let author: Author = response.json().await.expect("Failed to parse response");

    let fetched_author = app
        .author_repo
        .get(author.id)
        .await
        .expect("Failed to fetch author");

    assert_eq!(fetched_author.name, "Persistent Author");
}

#[tokio::test]
async fn getting_an_author_returns_a_200_for_valid_id() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let new_author = NewAuthor {
        name: "Fetchable Author".to_string(),
        created_at: None,
    };

    let create_response = client
        .post(app.api_url("/authors"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_author)
        .send()
        .await
        .expect("Failed to create author");

    let created_author: Author = create_response
        .json()
        .await
        .expect("Failed to parse response");

    let response = client
        .get(app.api_url(&format!("/authors/{}", created_author.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let author: Author = response.json().await.expect("Failed to parse response");
    assert_eq!(author.id, created_author.id);
    assert_eq!(author.name, "Fetchable Author");
}

#[tokio::test]
async fn listing_authors_returns_a_200_with_multiple_authors() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let author1 = NewAuthor {
        name: "First Author".to_string(),
        created_at: None,
    };

    let author2 = NewAuthor {
        name: "Second Author".to_string(),
        created_at: None,
    };

    client
        .post(app.api_url("/authors"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&author1)
        .send()
        .await
        .expect("Failed to create first author");

    client
        .post(app.api_url("/authors"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&author2)
        .send()
        .await
        .expect("Failed to create second author");

    let response = client
        .get(app.api_url("/authors"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let authors: Vec<Author> = response.json().await.expect("Failed to parse response");
    assert_eq!(authors.len(), 2);
}

#[tokio::test]
async fn updating_an_author_returns_a_200_for_valid_data() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let new_author = NewAuthor {
        name: "Original Name".to_string(),
        created_at: None,
    };

    let create_response = client
        .post(app.api_url("/authors"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&new_author)
        .send()
        .await
        .expect("Failed to create author");

    let created_author: Author = create_response
        .json()
        .await
        .expect("Failed to parse response");

    let update = UpdateAuthor {
        name: Some("Updated Name".to_string()),
        created_at: None,
    };

    let response = client
        .put(app.api_url(&format!("/authors/{}", created_author.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&update)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 200);

    let updated_author: Author = response.json().await.expect("Failed to parse response");
    assert_eq!(updated_author.name, "Updated Name");
}

#[tokio::test]
async fn updating_an_author_with_no_changes_returns_a_400() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let author = create_author_with_name(&app, "Test Author").await;

    let update = UpdateAuthor {
        name: None,
        created_at: None,
    };

    let response = client
        .put(app.api_url(&format!("/authors/{}", author.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&update)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn updating_a_nonexistent_author_returns_a_404() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let update = UpdateAuthor {
        name: Some("New Name".to_string()),
        created_at: None,
    };

    let response = client
        .put(app.api_url("/authors/999999"))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .json(&update)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn deleting_an_author_returns_a_204_for_valid_id() {
    let app = spawn_app_with_auth().await;
    let client = reqwest::Client::new();

    let author = create_author_with_name(&app, "To Be Deleted").await;

    let response = client
        .delete(app.api_url(&format!("/authors/{}", author.id)))
        .bearer_auth(app.auth_token.as_ref().unwrap())
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 204);

    let get_response = client
        .get(app.api_url(&format!("/authors/{}", author.id)))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(get_response.status(), 404);
}
