use anyhow::Result;

use super::BooklogClient;
use super::define_client_crud;
use crate::domain::books::readings::{NewReading, ReadingWithBook, UpdateReading};
use crate::domain::ids::{BookId, ReadingId};

pub struct ReadingsClient<'a> {
    client: &'a BooklogClient,
}

impl<'a> ReadingsClient<'a> {
    pub fn new(client: &'a BooklogClient) -> Self {
        Self { client }
    }

    define_client_crud!(
        entity_path: "api/v1/readings",
        id_type: ReadingId,
        entity_type: ReadingWithBook,
        new_type: NewReading,
        update_type: UpdateReading
    );

    pub async fn list(&self, book_id: Option<BookId>) -> Result<Vec<ReadingWithBook>> {
        let mut url = self.client.endpoint("api/v1/readings")?;
        if let Some(id) = book_id {
            url.query_pairs_mut()
                .append_pair("book_id", &id.to_string());
        }
        let response = self
            .client
            .request(reqwest::Method::GET, url)
            .send()
            .await?;
        self.client.handle_response(response).await
    }
}
