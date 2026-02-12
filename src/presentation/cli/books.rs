use anyhow::Result;
use clap::{Args, Subcommand};

use super::macros::{define_delete_command, define_get_command};
use super::parse_created_at;
use super::print_json;
use crate::domain::books::books::{AuthorRole, BookAuthor, NewBook, UpdateBook};
use crate::domain::ids::{AuthorId, BookId, GenreId};
use crate::infrastructure::client::BooklogClient;

#[derive(Debug, Subcommand)]
pub enum BookCommands {
    /// Add a new book
    Add(AddBookCommand),
    /// List all books
    List(ListBooksCommand),
    /// Get a book by ID
    Get(GetBookCommand),
    /// Update a book
    Update(UpdateBookCommand),
    /// Delete a book
    Delete(DeleteBookCommand),
}

pub async fn run(client: &BooklogClient, cmd: BookCommands) -> Result<()> {
    match cmd {
        BookCommands::Add(c) => add_book(client, c).await,
        BookCommands::List(c) => list_books(client, c).await,
        BookCommands::Get(c) => get_book(client, c).await,
        BookCommands::Update(c) => update_book(client, c).await,
        BookCommands::Delete(c) => delete_book(client, c).await,
    }
}

#[derive(Debug, Args)]
pub struct AddBookCommand {
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub isbn: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub page_count: Option<i32>,
    #[arg(long)]
    pub year_published: Option<i32>,
    #[arg(long)]
    pub publisher: Option<String>,
    #[arg(long)]
    pub language: Option<String>,
    #[arg(long)]
    pub primary_genre_id: Option<i64>,
    #[arg(long)]
    pub secondary_genre_id: Option<i64>,
    #[arg(long, value_delimiter = ',')]
    pub author_ids: Vec<i64>,
    /// Override creation timestamp (e.g. 2025-08-05T10:00:00Z or 2025-08-05)
    #[arg(long)]
    pub created_at: Option<String>,
}

pub async fn add_book(client: &BooklogClient, command: AddBookCommand) -> Result<()> {
    let created_at = command
        .created_at
        .map(|s| parse_created_at(&s))
        .transpose()?;
    let authors = command
        .author_ids
        .into_iter()
        .map(|id| BookAuthor {
            author_id: AuthorId::new(id),
            role: AuthorRole::default(),
        })
        .collect();
    let payload = NewBook {
        title: command.title,
        isbn: command.isbn,
        description: command.description,
        page_count: command.page_count,
        year_published: command.year_published,
        publisher: command.publisher,
        language: command.language,
        primary_genre_id: command.primary_genre_id.map(GenreId::new),
        secondary_genre_id: command.secondary_genre_id.map(GenreId::new),
        authors,
        created_at,
    };

    let book = client.books().create(&payload).await?;
    print_json(&book)
}

#[derive(Debug, Args)]
pub struct ListBooksCommand {
    #[arg(long)]
    pub author_id: Option<i64>,
}

pub async fn list_books(client: &BooklogClient, command: ListBooksCommand) -> Result<()> {
    let books = client
        .books()
        .list(command.author_id.map(AuthorId::new))
        .await?;
    print_json(&books)
}

define_get_command!(GetBookCommand, get_book, BookId, books);

#[derive(Debug, Args)]
pub struct UpdateBookCommand {
    #[arg(long)]
    pub id: i64,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub isbn: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub page_count: Option<i32>,
    #[arg(long)]
    pub year_published: Option<i32>,
    #[arg(long)]
    pub publisher: Option<String>,
    #[arg(long)]
    pub language: Option<String>,
    #[arg(long)]
    pub primary_genre_id: Option<i64>,
    #[arg(long)]
    pub secondary_genre_id: Option<i64>,
    #[arg(long, value_delimiter = ',')]
    pub author_ids: Option<Vec<i64>>,
    /// Override creation timestamp (e.g. 2025-08-05T10:00:00Z or 2025-08-05)
    #[arg(long)]
    pub created_at: Option<String>,
}

pub async fn update_book(client: &BooklogClient, command: UpdateBookCommand) -> Result<()> {
    let created_at = command
        .created_at
        .map(|s| parse_created_at(&s))
        .transpose()?;
    let authors = command.author_ids.map(|ids| {
        ids.into_iter()
            .map(|id| BookAuthor {
                author_id: AuthorId::new(id),
                role: AuthorRole::default(),
            })
            .collect()
    });
    let payload = UpdateBook {
        title: command.title,
        isbn: command.isbn,
        description: command.description,
        page_count: command.page_count,
        year_published: command.year_published,
        publisher: command.publisher,
        language: command.language,
        primary_genre_id: command.primary_genre_id.map(|id| Some(GenreId::new(id))),
        secondary_genre_id: command.secondary_genre_id.map(|id| Some(GenreId::new(id))),
        authors,
        created_at,
    };

    let book = client
        .books()
        .update(BookId::new(command.id), &payload)
        .await?;
    print_json(&book)
}

define_delete_command!(DeleteBookCommand, delete_book, BookId, books, "book");
