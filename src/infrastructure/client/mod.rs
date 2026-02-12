pub mod authors;
pub mod backup;
pub mod books;
pub mod genres;
pub mod readings;
pub mod tokens;
pub mod user_books;

/// Generates `create`, `get`, `update`, and `delete` methods for an entity client.
///
/// The `list` method is NOT generated because each entity has different filter parameters.
///
/// # Example
/// ```ignore
/// define_client_crud!(
///     entity_path: "api/v1/books",
///     id_type: BookId,
///     entity_type: BookWithAuthors,
///     new_type: NewBook,
///     update_type: UpdateBook
/// );
/// ```
macro_rules! define_client_crud {
    (
        entity_path: $path:expr,
        id_type: $id:ty,
        entity_type: $entity:ty,
        new_type: $new:ty,
        update_type: $update:ty
    ) => {
        pub async fn create(&self, payload: &$new) -> anyhow::Result<$entity> {
            let url = self.client.endpoint($path)?;
            let response = self
                .client
                .request(reqwest::Method::POST, url)
                .json(payload)
                .send()
                .await?;
            self.client.handle_response(response).await
        }

        pub async fn get(&self, id: $id) -> anyhow::Result<$entity> {
            let url = self.client.endpoint(&format!(concat!($path, "/{}"), id))?;
            let response = self
                .client
                .request(reqwest::Method::GET, url)
                .send()
                .await?;
            self.client.handle_response(response).await
        }

        pub async fn update(&self, id: $id, payload: &$update) -> anyhow::Result<$entity> {
            let url = self.client.endpoint(&format!(concat!($path, "/{}"), id))?;
            let response = self
                .client
                .request(reqwest::Method::PUT, url)
                .json(payload)
                .send()
                .await?;
            self.client.handle_response(response).await
        }

        pub async fn delete(&self, id: $id) -> anyhow::Result<()> {
            let url = self.client.endpoint(&format!(concat!($path, "/{}"), id))?;
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
    };
}

pub(crate) use define_client_crud;

use anyhow::{Context, Result, anyhow};
use reqwest::{Client, Url};

use crate::application::errors::ErrorResponse;

pub struct BooklogClient {
    base_url: Url,
    http: Client,
    token: Option<String>,
}

impl BooklogClient {
    pub fn new(base_url: Url) -> Result<Self> {
        let mut normalized = base_url;
        if !normalized.path().ends_with('/') {
            normalized.set_path(&format!("{}/", normalized.path().trim_end_matches('/')));
        }

        let token = std::env::var("BOOKLOG_TOKEN").ok();

        let http = Client::builder()
            .user_agent("booklog-cli/0.1")
            .build()
            .context("failed to configure HTTP client")?;

        Ok(Self {
            base_url: normalized,
            http,
            token,
        })
    }

    pub fn from_base_url(base_url: &str) -> Result<Self> {
        let url = Url::parse(base_url).with_context(|| format!("invalid API url: {base_url}"))?;
        Self::new(url)
    }

    pub fn backup(&self) -> backup::BackupClient<'_> {
        backup::BackupClient::new(self)
    }

    pub fn authors(&self) -> authors::AuthorsClient<'_> {
        authors::AuthorsClient::new(self)
    }

    pub fn books(&self) -> books::BooksClient<'_> {
        books::BooksClient::new(self)
    }

    pub fn tokens(&self) -> tokens::TokensClient<'_> {
        tokens::TokensClient::new(self)
    }

    pub fn readings(&self) -> readings::ReadingsClient<'_> {
        readings::ReadingsClient::new(self)
    }

    pub fn genres(&self) -> genres::GenresClient<'_> {
        genres::GenresClient::new(self)
    }

    pub fn user_books(&self) -> user_books::UserBooksClient<'_> {
        user_books::UserBooksClient::new(self)
    }

    pub(crate) fn endpoint(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .with_context(|| format!("invalid API path: {path}"))
    }

    /// Build a request with authentication if token is available
    pub(crate) fn request(&self, method: reqwest::Method, url: Url) -> reqwest::RequestBuilder {
        let mut request = self.http.request(method, url);
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        request
    }

    pub(crate) async fn handle_response<T>(&self, response: reqwest::Response) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        if response.status().is_success() {
            response
                .json::<T>()
                .await
                .context("failed to deserialize response body")
        } else {
            Err(self.response_error(response).await)
        }
    }

    pub(crate) async fn response_error(&self, response: reqwest::Response) -> anyhow::Error {
        let status = response.status();
        let bytes = response.bytes().await.unwrap_or_default();

        if let Ok(err) = serde_json::from_slice::<ErrorResponse>(&bytes) {
            return anyhow!("request failed ({status}): {}", err.message);
        }

        let message = String::from_utf8_lossy(&bytes);
        anyhow!("request failed ({status}): {message}")
    }
}
