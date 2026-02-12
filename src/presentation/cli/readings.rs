use anyhow::Result;
use clap::{Args, Subcommand};

use super::macros::{define_delete_command, define_get_command};
use super::parse_created_at;
use super::print_json;
use crate::domain::books::readings::{
    NewReading, QuickReview, ReadingFormat, ReadingStatus, UpdateReading,
};
use crate::domain::ids::{BookId, ReadingId, UserId};
use crate::infrastructure::client::BooklogClient;

#[derive(Debug, Subcommand)]
pub enum ReadingCommands {
    /// Add a new reading
    Add(AddReadingCommand),
    /// List all readings
    List(ListReadingsCommand),
    /// Get a reading by ID
    Get(GetReadingCommand),
    /// Update a reading
    Update(UpdateReadingCommand),
    /// Delete a reading
    Delete(DeleteReadingCommand),
}

pub async fn run(client: &BooklogClient, cmd: ReadingCommands) -> Result<()> {
    match cmd {
        ReadingCommands::Add(c) => add_reading(client, c).await,
        ReadingCommands::List(c) => list_readings(client, c).await,
        ReadingCommands::Get(c) => get_reading(client, c).await,
        ReadingCommands::Update(c) => update_reading(client, c).await,
        ReadingCommands::Delete(c) => delete_reading(client, c).await,
    }
}

#[derive(Debug, Args)]
pub struct AddReadingCommand {
    #[arg(long)]
    pub book_id: i64,
    #[arg(long, default_value = "reading")]
    pub status: String,
    /// Reading format: physical, ereader, or audiobook
    #[arg(long)]
    pub format: Option<String>,
    #[arg(long)]
    pub started_at: Option<String>,
    #[arg(long)]
    pub finished_at: Option<String>,
    #[arg(long)]
    pub rating: Option<f64>,
    /// Comma-separated quick review labels (e.g. "loved-it,page-turner")
    #[arg(long)]
    pub quick_reviews: Option<String>,
    /// Override creation timestamp (e.g. 2025-08-05T10:00:00Z or 2025-08-05)
    #[arg(long)]
    pub created_at: Option<String>,
}

pub async fn add_reading(client: &BooklogClient, command: AddReadingCommand) -> Result<()> {
    let status: ReadingStatus = command
        .status
        .parse()
        .map_err(|()| anyhow::anyhow!("invalid status: {}", command.status))?;
    let format = command
        .format
        .map(|s| {
            s.parse::<ReadingFormat>()
                .map_err(|()| anyhow::anyhow!("invalid format: {s}"))
        })
        .transpose()?;
    let started_at = command
        .started_at
        .map(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d"))
        .transpose()?;
    let finished_at = command
        .finished_at
        .map(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d"))
        .transpose()?;
    let created_at = command
        .created_at
        .map(|s| parse_created_at(&s))
        .transpose()?;

    // user_id is set by the server from the bearer token, placeholder here
    let payload = NewReading {
        user_id: UserId::new(0),
        book_id: BookId::new(command.book_id),
        status,
        format,
        started_at,
        finished_at,
        rating: command.rating,
        quick_reviews: command
            .quick_reviews
            .map(|s| {
                s.split(',')
                    .filter_map(|v| QuickReview::from_str_value(v.trim()))
                    .collect()
            })
            .unwrap_or_default(),
        created_at,
    };

    let reading = client.readings().create(&payload).await?;
    print_json(&reading)
}

#[derive(Debug, Args)]
pub struct ListReadingsCommand {
    #[arg(long)]
    pub book_id: Option<i64>,
}

pub async fn list_readings(client: &BooklogClient, command: ListReadingsCommand) -> Result<()> {
    let readings = client
        .readings()
        .list(command.book_id.map(BookId::new))
        .await?;
    print_json(&readings)
}

define_get_command!(GetReadingCommand, get_reading, ReadingId, readings);

#[derive(Debug, Args)]
pub struct UpdateReadingCommand {
    #[arg(long)]
    pub id: i64,
    #[arg(long)]
    pub status: Option<String>,
    /// Reading format: physical, ereader, or audiobook
    #[arg(long)]
    pub format: Option<String>,
    #[arg(long)]
    pub started_at: Option<String>,
    #[arg(long)]
    pub finished_at: Option<String>,
    #[arg(long)]
    pub rating: Option<f64>,
    /// Comma-separated quick review labels (e.g. "loved-it,page-turner")
    #[arg(long)]
    pub quick_reviews: Option<String>,
    /// Override creation timestamp (e.g. 2025-08-05T10:00:00Z or 2025-08-05)
    #[arg(long)]
    pub created_at: Option<String>,
}

pub async fn update_reading(client: &BooklogClient, command: UpdateReadingCommand) -> Result<()> {
    let status = command
        .status
        .map(|s| {
            s.parse::<ReadingStatus>()
                .map_err(|()| anyhow::anyhow!("invalid status: {s}"))
        })
        .transpose()?;
    let format = command
        .format
        .map(|s| {
            s.parse::<ReadingFormat>()
                .map_err(|()| anyhow::anyhow!("invalid format: {s}"))
        })
        .transpose()?;
    let started_at = command
        .started_at
        .map(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d"))
        .transpose()?;
    let finished_at = command
        .finished_at
        .map(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d"))
        .transpose()?;
    let created_at = command
        .created_at
        .map(|s| parse_created_at(&s))
        .transpose()?;

    let payload = UpdateReading {
        book_id: None,
        status,
        format,
        started_at,
        finished_at,
        rating: command.rating,
        quick_reviews: command.quick_reviews.map(|s| {
            s.split(',')
                .filter_map(|v| QuickReview::from_str_value(v.trim()))
                .collect()
        }),
        created_at,
    };

    let reading = client
        .readings()
        .update(ReadingId::new(command.id), &payload)
        .await?;
    print_json(&reading)
}

define_delete_command!(
    DeleteReadingCommand,
    delete_reading,
    ReadingId,
    readings,
    "reading"
);
