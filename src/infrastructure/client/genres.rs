use anyhow::Result;

use super::BooklogClient;
use super::define_client_crud;
use crate::domain::books::genres::{Genre, NewGenre, UpdateGenre};
use crate::domain::ids::GenreId;

pub struct GenresClient<'a> {
    client: &'a BooklogClient,
}

impl<'a> GenresClient<'a> {
    pub fn new(client: &'a BooklogClient) -> Self {
        Self { client }
    }

    define_client_crud!(
        entity_path: "api/v1/genres",
        id_type: GenreId,
        entity_type: Genre,
        new_type: NewGenre,
        update_type: UpdateGenre
    );

    pub async fn list(&self) -> Result<Vec<Genre>> {
        let url = self.client.endpoint("api/v1/genres")?;
        let response = self
            .client
            .request(reqwest::Method::GET, url)
            .send()
            .await?;
        self.client.handle_response(response).await
    }
}
