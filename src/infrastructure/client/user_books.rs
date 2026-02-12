use anyhow::Result;

use super::BooklogClient;
use crate::domain::ids::{BookId, UserBookId};
use crate::domain::user_books::{Shelf, UserBook};

pub struct UserBooksClient<'a> {
    client: &'a BooklogClient,
}

impl<'a> UserBooksClient<'a> {
    pub fn new(client: &'a BooklogClient) -> Self {
        Self { client }
    }

    #[allow(clippy::similar_names)] // self vs shelf
    pub async fn create(
        &self,
        book_id: BookId,
        shelf: Shelf,
        is_book_club: bool,
    ) -> Result<UserBook> {
        let url = self.client.endpoint("api/v1/user-books")?;
        let payload = serde_json::json!({
            "book_id": book_id,
            "shelf": shelf.as_str(),
            "book_club": is_book_club,
        });
        let response = self
            .client
            .request(reqwest::Method::POST, url)
            .json(&payload)
            .send()
            .await?;
        self.client.handle_response(response).await
    }

    pub async fn set_book_club(&self, id: UserBookId, is_book_club: bool) -> Result<UserBook> {
        let url = self.client.endpoint(&format!("api/v1/user-books/{id}"))?;
        let payload = serde_json::json!({
            "book_club": is_book_club,
        });
        let response = self
            .client
            .request(reqwest::Method::PATCH, url)
            .json(&payload)
            .send()
            .await?;
        self.client.handle_response(response).await
    }

    #[allow(clippy::similar_names)] // self vs shelf
    pub async fn move_shelf(&self, id: UserBookId, shelf: Shelf) -> Result<UserBook> {
        let url = self.client.endpoint(&format!("api/v1/user-books/{id}"))?;
        let payload = serde_json::json!({
            "shelf": shelf.as_str(),
        });
        let response = self
            .client
            .request(reqwest::Method::PUT, url)
            .json(&payload)
            .send()
            .await?;
        self.client.handle_response(response).await
    }

    pub async fn delete(&self, id: UserBookId) -> Result<()> {
        let url = self.client.endpoint(&format!("api/v1/user-books/{id}"))?;
        let response = self
            .client
            .request(reqwest::Method::DELETE, url)
            .send()
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(self.client.response_error(response).await)
        }
    }

    pub async fn list(&self) -> Result<Vec<UserBook>> {
        let url = self.client.endpoint("api/v1/user-books")?;
        let response = self
            .client
            .request(reqwest::Method::GET, url)
            .send()
            .await?;
        self.client.handle_response(response).await
    }
}
