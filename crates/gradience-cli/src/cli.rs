use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "gradience")]
#[command(about = "Gradience Wallet - Agent Wallet Orchestration Platform")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Authenticate with your vault
    Auth {
        #[command(subcommand)]
        cmd: AuthCommands,
    },
    /// Manage agents and wallets
    Agent {
        #[command(subcommand)]
        cmd: AgentCommands,
    },
    /// Manage policies
    Policy {
        #[command(subcommand)]
        cmd: PolicyCommands,
    },
    /// Manage API keys
    ApiKey {
        #[command(subcommand)]
        cmd: ApiKeyCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum AuthCommands {
    /// Login to your vault
    Login,
}

#[derive(Subcommand, Debug)]
pub enum AgentCommands {
    /// Create a new wallet
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        workspace: Option<String>,
    },
    /// List wallets
    List,
    /// Check balance
    Balance {
        wallet_id: String,
        #[arg(long)]
        chain: Option<String>,
    },
    /// Send funds from wallet
    Fund {
        wallet_id: String,
        amount: String,
        #[arg(long)]
        chain: Option<String>,
        #[arg(long)]
        to: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum PolicyCommands {
    /// Set policy for a wallet
    Set {
        wallet_id: String,
        #[arg(long)]
        file: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ApiKeyCommands {
    /// Create an API key for a wallet
    Create {
        wallet_id: String,
        #[arg(long)]
        name: String,
    },
    /// Revoke an API key
    Revoke {
        key_id: String,
    },
    /// List API keys for a wallet
    List {
        wallet_id: String,
    },
}
