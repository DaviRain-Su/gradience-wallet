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
    /// DEX operations (quote and swap)
    Dex {
        #[command(subcommand)]
        cmd: DexCommands,
    },
    /// Audit log queries and verification
    Audit {
        #[command(subcommand)]
        cmd: AuditCommands,
    },
    /// Team and workspace management
    Team {
        #[command(subcommand)]
        cmd: TeamCommands,
    },
    /// AI Gateway commands
    Ai {
        #[command(subcommand)]
        cmd: AiCommands,
    },
    /// MCP server management
    Mcp {
        #[command(subcommand)]
        cmd: McpCommands,
    },
    /// Execute x402 payment
    Pay {
        wallet_id: String,
        recipient: String,
        amount: String,
        #[arg(long)]
        token: String,
        #[arg(long)]
        chain: Option<String>,
        #[arg(long)]
        deadline: Option<u64>,
    },
    /// Start the local web UI and API server
    Start,
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
    /// Approve a policy warning ticket
    Approve {
        approval_id: String,
    },
    /// Reject a policy warning ticket
    Reject {
        approval_id: String,
    },
    /// List pending policy approvals
    ListApprovals {
        wallet_id: Option<String>,
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

#[derive(Subcommand, Debug)]
pub enum DexCommands {
    /// Get swap quote
    Quote {
        wallet_id: String,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: String,
    },
    /// Execute swap
    Swap {
        wallet_id: String,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum AuditCommands {
    /// List audit logs for a wallet
    List {
        wallet_id: String,
    },
    /// Verify audit chain integrity
    Verify {
        wallet_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum TeamCommands {
    /// Create a workspace
    CreateWorkspace {
        #[arg(long)]
        name: String,
    },
    /// Invite member to workspace
    Invite {
        workspace_id: String,
        user_email: String,
        #[arg(long, value_enum)]
        role: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum AiCommands {
    /// Query AI balance
    Balance {
        wallet_id: String,
    },
    /// Generate text via LLM
    Generate {
        wallet_id: String,
        prompt: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum McpCommands {
    /// Start the MCP stdio server
    Serve,
    /// Sign a transaction via MCP tool
    SignTx {
        wallet_id: String,
        chain_id: String,
        to: String,
        amount: String,
    },
    /// Get balance via MCP tool
    Balance {
        wallet_id: String,
        chain_id: String,
    },
}
