use alloy::primitives::Address;

/// AA configuration for a given EVM chain.
#[derive(Debug, Clone)]
pub struct AaConfig {
    pub chain_id: u64,
    pub entry_point_v06: Address,
    /// Placeholder until a verified factory is deployed on-chain.
    pub simple_account_factory: Address,
    pub bundler_url: &'static str,
}

impl AaConfig {
    pub fn for_chain(chain_id: u64) -> Option<Self> {
        match chain_id {
            // Base Sepolia
            84532 => Some(Self {
                chain_id: 84532,
                entry_point_v06: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
                    .parse()
                    .unwrap(),
                simple_account_factory: "0x9406Cc6185a346906296840746125a0E44976454"
                    .parse()
                    .unwrap(),
                bundler_url: "https://bundler.particle.network",
            }),
            // X Layer Mainnet
            196 => Some(Self {
                chain_id: 196,
                entry_point_v06: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
                    .parse()
                    .unwrap(),
                // TODO: deploy or verify an official SimpleAccountFactory on X Layer
                simple_account_factory: "0x0000000000000000000000000000000000000000"
                    .parse()
                    .unwrap(),
                bundler_url: "https://bundler.particle.network",
            }),
            _ => None,
        }
    }
}
