use thiserror::Error;

pub type Result<T> = std::result::Result<T, GradienceError>;

#[derive(Debug, Error)]
pub enum GradienceError {
    #[error("Wallet not found: {0}")]
    WalletNotFound(String),
    #[error("Policy denied: {0}")]
    PolicyDenied(String),
    #[error("Insufficient balance")]
    InsufficientBalance,
    #[error("Invalid chain: {0}")]
    InvalidChain(String),
    #[error("OWS error: {0}")]
    Ows(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Signature error: {0}")]
    Signature(String),
    #[error("Invalid credential: {0}")]
    InvalidCredential(String),
}
