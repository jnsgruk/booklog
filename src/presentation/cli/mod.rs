pub mod authors;
pub mod backup;
pub mod books;
pub mod genres;
mod macros;
pub mod readings;
pub mod timeline;
pub mod tokens;
pub mod user_books;

use std::net::SocketAddr;

use chrono::{DateTime, NaiveDate, Utc};

use authors::AuthorCommands;
use backup::{BackupCommand, RestoreCommand};
use books::BookCommands;
use clap::{Args, Parser, Subcommand};
use genres::GenreCommands;
use readings::ReadingCommands;
use timeline::TimelineCommands;
use tokens::TokenCommands;
use user_books::UserBookCommands;

#[derive(Debug, Parser)]
#[command(author, version, about = "Track books and build a personal library", long_about = None)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        env = "BOOKLOG_URL",
        default_value = "http://localhost:3000"
    )]
    pub api_url: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Run the HTTP server
    Serve(ServeCommand),

    /// Manage authors
    Author {
        #[command(subcommand)]
        command: AuthorCommands,
    },

    /// Manage books
    Book {
        #[command(subcommand)]
        command: BookCommands,
    },

    /// Manage genres
    Genre {
        #[command(subcommand)]
        command: GenreCommands,
    },

    /// Manage library entries
    Reading {
        #[command(subcommand)]
        command: ReadingCommands,
    },

    /// Manage library and wishlist
    #[command(name = "user-book")]
    UserBook {
        #[command(subcommand)]
        command: UserBookCommands,
    },

    /// Manage API tokens
    Token {
        #[command(subcommand)]
        command: TokenCommands,
    },

    /// Manage timeline events
    Timeline {
        #[command(subcommand)]
        command: TimelineCommands,
    },

    /// Back up all book data to JSON (stdout)
    Backup(BackupCommand),

    /// Restore book data from a JSON backup file
    Restore(RestoreCommand),
}

#[derive(Debug, Args)]
pub struct ServeCommand {
    #[arg(
        long,
        env = "BOOKLOG_DATABASE_URL",
        default_value = "sqlite://booklog.db"
    )]
    pub database_url: String,

    #[arg(long, env = "BOOKLOG_BIND_ADDRESS", default_value = "127.0.0.1:3000")]
    pub bind_address: SocketAddr,

    #[arg(long, env = "BOOKLOG_RP_ID", default_value = "localhost")]
    pub rp_id: String,

    #[arg(
        long,
        env = "BOOKLOG_RP_ORIGIN",
        default_value = "http://localhost:3000"
    )]
    pub rp_origin: String,

    #[arg(long, env = "BOOKLOG_INSECURE_COOKIES")]
    pub insecure_cookies: bool,

    #[arg(long, env = "BOOKLOG_OPENROUTER_API_KEY")]
    pub openrouter_api_key: Option<String>,

    #[arg(
        long,
        env = "BOOKLOG_OPENROUTER_MODEL",
        default_value = "openrouter/free"
    )]
    pub openrouter_model: String,
}

pub fn parse_created_at(value: &str) -> anyhow::Result<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Ok(dt.with_timezone(&Utc));
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return Ok(date.and_time(chrono::NaiveTime::MIN).and_utc());
    }
    anyhow::bail!(
        "invalid date format: expected RFC 3339 (e.g. 2025-08-05T10:00:00Z) or YYYY-MM-DD"
    )
}

pub(crate) fn print_json<T>(value: &T) -> anyhow::Result<()>
where
    T: serde::Serialize,
{
    let rendered = serde_json::to_string_pretty(value)?;
    println!("{rendered}");
    Ok(())
}
