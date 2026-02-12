use anyhow::Result;
use clap::{Args, Subcommand};

use super::macros::{define_delete_command, define_get_command};
use super::parse_created_at;
use super::print_json;
use crate::domain::books::genres::{NewGenre, UpdateGenre};
use crate::domain::ids::GenreId;
use crate::infrastructure::client::BooklogClient;

#[derive(Debug, Subcommand)]
pub enum GenreCommands {
    /// Add a new genre
    Add(AddGenreCommand),
    /// List all genres
    List,
    /// Get a genre by ID
    Get(GetGenreCommand),
    /// Update a genre
    Update(UpdateGenreCommand),
    /// Delete a genre
    Delete(DeleteGenreCommand),
}

pub async fn run(client: &BooklogClient, cmd: GenreCommands) -> Result<()> {
    match cmd {
        GenreCommands::Add(c) => add_genre(client, c).await,
        GenreCommands::List => list_genres(client).await,
        GenreCommands::Get(c) => get_genre(client, c).await,
        GenreCommands::Update(c) => update_genre(client, c).await,
        GenreCommands::Delete(c) => delete_genre(client, c).await,
    }
}

#[derive(Debug, Args)]
pub struct AddGenreCommand {
    #[arg(long)]
    pub name: String,
    /// Override creation timestamp (e.g. 2025-08-05T10:00:00Z or 2025-08-05)
    #[arg(long)]
    pub created_at: Option<String>,
}

pub async fn add_genre(client: &BooklogClient, command: AddGenreCommand) -> Result<()> {
    let created_at = command
        .created_at
        .map(|s| parse_created_at(&s))
        .transpose()?;
    let payload = NewGenre {
        name: command.name,
        created_at,
    };

    let genre = client.genres().create(&payload).await?;
    print_json(&genre)
}

pub async fn list_genres(client: &BooklogClient) -> Result<()> {
    let genres = client.genres().list().await?;
    print_json(&genres)
}

define_get_command!(GetGenreCommand, get_genre, GenreId, genres);

#[derive(Debug, Args)]
pub struct UpdateGenreCommand {
    #[arg(long)]
    pub id: i64,
    #[arg(long)]
    pub name: Option<String>,
    /// Override creation timestamp (e.g. 2025-08-05T10:00:00Z or 2025-08-05)
    #[arg(long)]
    pub created_at: Option<String>,
}

pub async fn update_genre(client: &BooklogClient, command: UpdateGenreCommand) -> Result<()> {
    let created_at = command
        .created_at
        .map(|s| parse_created_at(&s))
        .transpose()?;
    let payload = UpdateGenre {
        name: command.name,
        created_at,
    };

    let genre = client
        .genres()
        .update(GenreId::new(command.id), &payload)
        .await?;
    print_json(&genre)
}

define_delete_command!(DeleteGenreCommand, delete_genre, GenreId, genres, "genre");
