use anyhow::Result;
use clap::{Args, Subcommand};

use super::macros::{define_delete_command, define_get_command};
use super::parse_created_at;
use super::print_json;
use crate::domain::books::authors::{NewAuthor, UpdateAuthor};
use crate::domain::ids::AuthorId;
use crate::infrastructure::client::BooklogClient;

#[derive(Debug, Subcommand)]
pub enum AuthorCommands {
    /// Add a new author
    Add(AddAuthorCommand),
    /// List all authors
    List,
    /// Get an author by ID
    Get(GetAuthorCommand),
    /// Update an author
    Update(UpdateAuthorCommand),
    /// Delete an author
    Delete(DeleteAuthorCommand),
}

pub async fn run(client: &BooklogClient, cmd: AuthorCommands) -> Result<()> {
    match cmd {
        AuthorCommands::Add(c) => add_author(client, c).await,
        AuthorCommands::List => list_authors(client).await,
        AuthorCommands::Get(c) => get_author(client, c).await,
        AuthorCommands::Update(c) => update_author(client, c).await,
        AuthorCommands::Delete(c) => delete_author(client, c).await,
    }
}

#[derive(Debug, Args)]
pub struct AddAuthorCommand {
    #[arg(long)]
    pub name: String,
    /// Override creation timestamp (e.g. 2025-08-05T10:00:00Z or 2025-08-05)
    #[arg(long)]
    pub created_at: Option<String>,
}

pub async fn add_author(client: &BooklogClient, command: AddAuthorCommand) -> Result<()> {
    let created_at = command
        .created_at
        .map(|s| parse_created_at(&s))
        .transpose()?;
    let payload = NewAuthor {
        name: command.name,
        created_at,
    };

    let author = client.authors().create(&payload).await?;
    print_json(&author)
}

pub async fn list_authors(client: &BooklogClient) -> Result<()> {
    let authors = client.authors().list().await?;
    print_json(&authors)
}

define_get_command!(GetAuthorCommand, get_author, AuthorId, authors);

#[derive(Debug, Args)]
pub struct UpdateAuthorCommand {
    #[arg(long)]
    pub id: i64,
    #[arg(long)]
    pub name: Option<String>,
    /// Override creation timestamp (e.g. 2025-08-05T10:00:00Z or 2025-08-05)
    #[arg(long)]
    pub created_at: Option<String>,
}

pub async fn update_author(client: &BooklogClient, command: UpdateAuthorCommand) -> Result<()> {
    let created_at = command
        .created_at
        .map(|s| parse_created_at(&s))
        .transpose()?;
    let payload = UpdateAuthor {
        name: command.name,
        created_at,
    };

    let author = client
        .authors()
        .update(AuthorId::new(command.id), &payload)
        .await?;
    print_json(&author)
}

define_delete_command!(
    DeleteAuthorCommand,
    delete_author,
    AuthorId,
    authors,
    "author"
);
