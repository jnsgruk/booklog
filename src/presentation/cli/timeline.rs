use clap::Subcommand;

use crate::infrastructure::client::BooklogClient;

#[derive(Debug, Subcommand)]
pub enum TimelineCommands {
    /// Rebuild all timeline event snapshots
    Rebuild,
}

pub async fn run(client: &BooklogClient, command: TimelineCommands) -> anyhow::Result<()> {
    match command {
        TimelineCommands::Rebuild => {
            client.timeline().rebuild().await?;
            eprintln!("Timeline rebuild triggered.");
            Ok(())
        }
    }
}
