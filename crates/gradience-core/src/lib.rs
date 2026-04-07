pub mod ai;
pub mod config;
pub mod error;
pub mod identity;
pub mod wallet;
pub mod policy;
pub mod dex;
pub mod payment;
pub mod audit;
pub mod ows;
pub mod rpc;
pub mod team;

pub use error::{GradienceError, Result};


#[cfg(test)]
mod tests;
