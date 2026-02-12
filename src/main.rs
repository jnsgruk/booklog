use anyhow::Result;
use booklog::application::{ServerConfig, serve};
use booklog::infrastructure::backup::BackupData;
use booklog::infrastructure::client::BooklogClient;
use booklog::presentation::cli::{
    Cli, Commands, ServeCommand, authors, books, genres, readings, tokens, user_books,
};
use clap::Parser;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present (before clap parses env vars)
    let _ = dotenvy::dotenv();

    init_tracing();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve(cmd) => run_server(cmd).await,
        Commands::Author { command } => {
            let client = BooklogClient::from_base_url(&cli.api_url)?;
            authors::run(&client, command).await
        }
        Commands::Book { command } => {
            let client = BooklogClient::from_base_url(&cli.api_url)?;
            books::run(&client, command).await
        }
        Commands::Genre { command } => {
            let client = BooklogClient::from_base_url(&cli.api_url)?;
            genres::run(&client, command).await
        }
        Commands::Reading { command } => {
            let client = BooklogClient::from_base_url(&cli.api_url)?;
            readings::run(&client, command).await
        }
        Commands::UserBook { command } => {
            let client = BooklogClient::from_base_url(&cli.api_url)?;
            user_books::run(&client, command).await
        }
        Commands::Token { command } => {
            let client = BooklogClient::from_base_url(&cli.api_url)?;
            tokens::run(&client, command).await
        }
        Commands::Backup(_cmd) => {
            let client = BooklogClient::from_base_url(&cli.api_url)?;
            let data = client.backup().export().await?;
            let json = serde_json::to_string_pretty(&data)?;
            println!("{json}");
            Ok(())
        }
        Commands::Restore(cmd) => {
            let contents = std::fs::read_to_string(&cmd.file)?;
            let data: BackupData = serde_json::from_str(&contents)?;
            let client = BooklogClient::from_base_url(&cli.api_url)?;
            client.backup().restore(&data).await?;
            eprintln!("Restore complete.");
            Ok(())
        }
    }
}

async fn run_server(command: ServeCommand) -> Result<()> {
    let rp_id = command.rp_id;
    let rp_origin = command.rp_origin;

    let insecure_cookies = command.insecure_cookies
        || (rp_id == "localhost" && rp_origin.starts_with("http://localhost"));
    if insecure_cookies {
        tracing::warn!(
            "insecure cookies enabled for development/demo setup - do not use in production"
        );
    }

    let openrouter_api_key = command.openrouter_api_key.unwrap_or_default();

    booklog::set_base_url(rp_origin.clone());

    let config = ServerConfig {
        bind_address: command.bind_address,
        database_url: command.database_url,
        rp_id,
        rp_origin,
        insecure_cookies,
        openrouter_api_key,
        openrouter_model: command.openrouter_model,
    };

    serve(config).await
}

#[allow(clippy::expect_used)] // Startup: panicking is appropriate if logging cannot be initialized
fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let use_json = std::env::var("RUST_LOG_FORMAT").is_ok_and(|v| v.eq_ignore_ascii_case("json"));

    let registry = tracing_subscriber::registry().with(env_filter);

    if use_json {
        registry
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        registry
            .with(tracing_subscriber::fmt::layer().compact())
            .init();
    }
}
