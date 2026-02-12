use anyhow::Result;

use super::BooklogClient;
use super::define_client_crud;
use crate::domain::books::books::{BookWithAuthors, NewBook, UpdateBook};
use crate::domain::ids::{AuthorId, BookId};

pub struct BooksClient<'a> {
    client: &'a BooklogClient,
}

impl<'a> BooksClient<'a> {
    pub fn new(client: &'a BooklogClient) -> Self {
        Self { client }
    }

    define_client_crud!(
        entity_path: "api/v1/books",
        id_type: BookId,
        entity_type: BookWithAuthors,
        new_type: NewBook,
        update_type: UpdateBook
    );

    pub async fn list(&self, author_id: Option<AuthorId>) -> Result<Vec<BookWithAuthors>> {
        let mut url = self.client.endpoint("api/v1/books")?;
        if let Some(id) = author_id {
            url.query_pairs_mut()
                .append_pair("author_id", &id.to_string());
        }
        let response = self
            .client
            .request(reqwest::Method::GET, url)
            .send()
            .await?;
        self.client.handle_response(response).await
    }
}
