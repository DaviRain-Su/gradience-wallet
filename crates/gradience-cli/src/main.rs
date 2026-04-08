mod cli;
mod commands;
mod context;

use clap::Parser;
use cli::{Cli, Commands, AuthCommands, AgentCommands, PolicyCommands, ApiKeyCommands, DexCommands, AuditCommands, TeamCommands, AiCommands, McpCommands};
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

    match &cli.command {
        Commands::Start => return commands::start::run().await,
        _ => {}
    }

    let ctx = context::AppContext::new(&db_path, data_dir, vault_dir).await?;

    match cli.command {
        Commands::Auth { cmd } => match cmd {
            AuthCommands::Login => commands::auth::login(&ctx).await,
            AuthCommands::LocalUnlock => commands::auth::local_unlock(&ctx).await,
            AuthCommands::Whoami => commands::auth::whoami(&ctx).await,
        },
        Commands::Agent { cmd } => match cmd {
            AgentCommands::Create { name, workspace } => {
                commands::agent::create(&ctx, name, workspace).await
            }
            AgentCommands::List => commands::agent::list(&ctx).await,
            AgentCommands::Balance { wallet_id, chain } => {
                commands::agent::balance(&ctx, wallet_id, chain).await
            }
            AgentCommands::Fund { wallet_id, amount, chain, to } => {
                commands::agent::fund(&ctx, wallet_id, amount, chain, to).await
            }
        },
        Commands::Policy { cmd } => match cmd {
            PolicyCommands::Set { wallet_id, file } => {
                commands::policy::set(&ctx, wallet_id, file).await
            }
            PolicyCommands::Approve { approval_id } => {
                commands::policy::approve(&ctx, approval_id).await
            }
            PolicyCommands::Reject { approval_id } => {
                commands::policy::reject(&ctx, approval_id).await
            }
            PolicyCommands::ListApprovals { wallet_id } => {
                commands::policy::list_approvals(&ctx, wallet_id).await
            }
        },
        Commands::ApiKey { cmd } => match cmd {
            ApiKeyCommands::Create { wallet_id, name } => {
                commands::api_key::create(&ctx, wallet_id, name).await
            }
            ApiKeyCommands::Revoke { key_id } => {
                commands::api_key::revoke(&ctx, key_id).await
            }
            ApiKeyCommands::List { wallet_id } => {
                commands::api_key::list(&ctx, wallet_id).await
            }
        },
        Commands::Dex { cmd } => match cmd {
            DexCommands::Quote { wallet_id, from, to, amount, chain } => {
                commands::dex::quote(&ctx, wallet_id, from, to, amount, chain).await
            }
            DexCommands::Swap { wallet_id, from, to, amount, chain } => {
                commands::dex::swap(&ctx, wallet_id, from, to, amount, chain).await
            }
        },
        Commands::Audit { cmd } => match cmd {
            AuditCommands::List { wallet_id } => {
                commands::audit::list(&ctx, wallet_id).await
            }
            AuditCommands::Verify { wallet_id } => {
                commands::audit::verify(&ctx, wallet_id).await
            }
            AuditCommands::Export { wallet_id, format, output } => {
                let fmt_str = match format {
                    crate::cli::AuditFormat::Csv => "csv",
                    crate::cli::AuditFormat::Json => "json",
                };
                commands::audit::export(&ctx, wallet_id, fmt_str, output).await
            }
        },
        Commands::Team { cmd } => match cmd {
            TeamCommands::CreateWorkspace { name } => {
                commands::team::create_workspace(&ctx, name).await
            }
            TeamCommands::Invite { workspace_id, user_email, role } => {
                commands::team::invite(&ctx, workspace_id, user_email, role).await
            }
            TeamCommands::BudgetSet { workspace_id, amount, token, chain_id, period } => {
                commands::team::budget_set(&ctx, workspace_id, amount, token, chain_id, period).await
            }
            TeamCommands::BudgetStatus { workspace_id, token, chain_id, period } => {
                commands::team::budget_status(&ctx, workspace_id, token, chain_id, period).await
            }
        },
        Commands::Ai { cmd } => match cmd {
            AiCommands::Balance { wallet_id } => {
                commands::ai::balance(&ctx, wallet_id).await
            }
            AiCommands::Generate { wallet_id, prompt } => {
                commands::ai::generate(&ctx, wallet_id, prompt).await
            }
        },
        Commands::Mcp { cmd } => match cmd {
            McpCommands::Serve => commands::mcp::serve().await,
            McpCommands::SignTx { wallet_id, chain_id, to, amount } => {
                commands::mcp::sign_tx(&ctx, wallet_id, chain_id, to, amount).await
            }
            McpCommands::Balance { wallet_id, chain_id } => {
                commands::mcp::balance(&ctx, wallet_id, chain_id).await
            }
        },
        Commands::Pay { wallet_id, recipient, amount, token, chain, deadline } => {
            commands::pay::x402(&ctx, wallet_id, recipient, amount, token, chain, deadline).await
        }
        Commands::Start => {
            commands::start::run().await
        }
    }
}
