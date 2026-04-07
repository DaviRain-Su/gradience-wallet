mod cli;
mod commands;
mod context;

use clap::Parser;
use cli::{Cli, Commands, AuthCommands, AgentCommands, PolicyCommands};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let data_dir = if let Ok(dir) = std::env::var("GRADIENCE_DATA_DIR") {
        PathBuf::from(dir)
    } else {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".gradience")
    };
    let vault_dir = data_dir.join("vault");
    let db_path = format!(
        "sqlite:///{}?mode=rwc",
        data_dir.join("gradience.db").to_string_lossy().trim_start_matches('/')
    );

    let ctx = context::AppContext::new(&db_path, vault_dir).await?;

    match cli.command {
        Commands::Auth { cmd } => match cmd {
            AuthCommands::Login => commands::auth::login(&ctx).await,
        },
        Commands::Agent { cmd } => match cmd {
            AgentCommands::Create { name, workspace } => {
                commands::agent::create(&ctx, name, workspace).await
            }
            AgentCommands::List => commands::agent::list(&ctx).await,
            AgentCommands::Balance { wallet_id, chain } => {
                commands::agent::balance(&ctx, wallet_id, chain).await
            }
            AgentCommands::Fund { wallet_id, amount, chain } => {
                commands::agent::fund(&ctx, wallet_id, amount, chain).await
            }
        },
        Commands::Policy { cmd } => match cmd {
            PolicyCommands::Set { wallet_id, file } => {
                commands::policy::set(&ctx, wallet_id, file).await
            }
        },
    }
}
