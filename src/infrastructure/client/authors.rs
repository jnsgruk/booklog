use anyhow::Result;

use super::BooklogClient;
use super::define_client_crud;
use crate::domain::books::authors::{Author, NewAuthor, UpdateAuthor};
use crate::domain::ids::AuthorId;

pub struct AuthorsClient<'a> {
    client: &'a BooklogClient,
}

impl<'a> AuthorsClient<'a> {
    pub fn new(client: &'a BooklogClient) -> Self {
        Self { client }
    }

    define_client_crud!(
        entity_path: "api/v1/authors",
        id_type: AuthorId,
        entity_type: Author,
        new_type: NewAuthor,
        update_type: UpdateAuthor
    );

    pub async fn list(&self) -> Result<Vec<Author>> {
        let url = self.client.endpoint("api/v1/authors")?;
        let response = self
            .client
            .request(reqwest::Method::GET, url)
            .send()
            .await?;
        self.client.handle_response(response).await
    }
}
