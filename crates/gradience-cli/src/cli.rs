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
    /// Fund wallet (demo/mock)
    Fund {
        wallet_id: String,
        amount: String,
        #[arg(long)]
        chain: Option<String>,
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
