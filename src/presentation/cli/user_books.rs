use anyhow::Result;
use clap::{Args, Subcommand};

use super::print_json;
use crate::domain::ids::BookId;
use crate::domain::user_books::Shelf;
use crate::infrastructure::client::BooklogClient;

#[derive(Debug, Subcommand)]
pub enum UserBookCommands {
    /// Add a book to the library or wishlist
    Add(AddUserBookCommand),
    /// List all library/wishlist entries
    List(ListUserBooksCommand),
    /// Move a book between shelves
    Move(MoveUserBookCommand),
    /// Set or clear the book club flag
    SetBookClub(SetBookClubCommand),
    /// Remove a book from the library/wishlist
    Remove(RemoveUserBookCommand),
}

pub async fn run(client: &BooklogClient, cmd: UserBookCommands) -> Result<()> {
    match cmd {
        UserBookCommands::Add(c) => add_user_book(client, c).await,
        UserBookCommands::List(c) => list_user_books(client, c).await,
        UserBookCommands::Move(c) => move_user_book(client, c).await,
        UserBookCommands::SetBookClub(c) => set_book_club(client, c).await,
        UserBookCommands::Remove(c) => remove_user_book(client, c).await,
    }
}

#[derive(Debug, Args)]
pub struct AddUserBookCommand {
    #[arg(long)]
    pub book_id: i64,
    /// Shelf: library (default) or wishlist
    #[arg(long, default_value = "library")]
    pub shelf: String,
    /// Mark as a book club pick
    #[arg(long, default_value_t = false)]
    pub book_club: bool,
}

pub async fn add_user_book(client: &BooklogClient, command: AddUserBookCommand) -> Result<()> {
    let shelf: Shelf = command
        .shelf
        .parse()
        .map_err(|()| anyhow::anyhow!("invalid shelf: {}", command.shelf))?;

    let user_book = client
        .user_books()
        .create(BookId::new(command.book_id), shelf, command.book_club)
        .await?;
    print_json(&user_book)
}

#[derive(Debug, Args)]
pub struct ListUserBooksCommand {}

pub async fn list_user_books(client: &BooklogClient, _command: ListUserBooksCommand) -> Result<()> {
    let user_books = client.user_books().list().await?;
    print_json(&user_books)
}

#[derive(Debug, Args)]
pub struct MoveUserBookCommand {
    #[arg(long)]
    pub id: i64,
    /// Target shelf: library or wishlist
    #[arg(long)]
    pub shelf: String,
}

pub async fn move_user_book(client: &BooklogClient, command: MoveUserBookCommand) -> Result<()> {
    let shelf: Shelf = command
        .shelf
        .parse()
        .map_err(|()| anyhow::anyhow!("invalid shelf: {}", command.shelf))?;

    let user_book = client
        .user_books()
        .move_shelf(crate::domain::ids::UserBookId::new(command.id), shelf)
        .await?;
    print_json(&user_book)
}

#[derive(Debug, Args)]
pub struct SetBookClubCommand {
    #[arg(long)]
    pub id: i64,
    /// Set to true or false
    #[arg(long)]
    pub book_club: bool,
}

pub async fn set_book_club(client: &BooklogClient, command: SetBookClubCommand) -> Result<()> {
    let user_book = client
        .user_books()
        .set_book_club(
            crate::domain::ids::UserBookId::new(command.id),
            command.book_club,
        )
        .await?;
    print_json(&user_book)
}

#[derive(Debug, Args)]
pub struct RemoveUserBookCommand {
    #[arg(long)]
    pub id: i64,
}

pub async fn remove_user_book(
    client: &BooklogClient,
    command: RemoveUserBookCommand,
) -> Result<()> {
    client
        .user_books()
        .delete(crate::domain::ids::UserBookId::new(command.id))
        .await?;
    println!("Deleted user book {}", command.id);
    Ok(())
}
