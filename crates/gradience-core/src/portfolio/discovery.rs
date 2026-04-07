use crate::error::{GradienceError, Result};
use crate::rpc::evm::EvmRpcClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAsset {
    pub chain_id: String,
    pub address: String,
    pub token_address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub balance: String,
    pub balance_formatted: String,
}

#[derive(Debug, Clone)]
pub struct KnownToken {
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
}

fn known_tokens_for_chain(chain_id: &str) -> Vec<KnownToken> {
    let mut tokens = Vec::new();
    // Stablecoins + wrapped
    tokens.push(KnownToken { address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(), symbol: "USDC".into(), name: "USD Coin".into(), decimals: 6 });
    tokens.push(KnownToken { address: "0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb".into(), symbol: "DAI".into(),  name: "Dai Stablecoin".into(), decimals: 18 });
    tokens.push(KnownToken { address: "0x4200000000000000000000000000000000000006".into(), symbol: "WETH".into(), name: "Wrapped Ether".into(), decimals: 18 });

    let eth_common = vec![
        KnownToken { address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".into(), symbol: "USDC".into(), name: "USD Coin".into(), decimals: 6 },
        KnownToken { address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".into(), symbol: "USDT".into(), name: "Tether USD".into(), decimals: 6 },
        KnownToken { address: "0x6B175474E89094C44Da98b954EedeAC495271d0F".into(), symbol: "DAI".into(),  name: "Dai Stablecoin".into(), decimals: 18 },
        KnownToken { address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".into(), symbol: "WETH".into(), name: "Wrapped Ether".into(), decimals: 18 },
        KnownToken { address: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599".into(), symbol: "WBTC".into(), name: "Wrapped BTC".into(), decimals: 8 },
    ];

    match chain_id {
        "eip155:8453" => tokens, // Base tokens already pushed above
        "eip155:1" => eth_common,
        "eip155:137" => {
            let mut t = eth_common.clone();
            t[0].address = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".into(); // USDC Polygon
            t[1].address = "0xc2132D05D31c914a87C6611C10748AEb04B58e8F".into(); // USDT Polygon
            t[2].address = "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063".into(); // DAI Polygon
            t[3].address = "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619".into(); // WETH Polygon
            t[4].address = "0x1BFD67037B42Cf73acF2047067bd4F2C47D9BfD6".into(); // WBTC Polygon
            t
        }
        "eip155:42161" => {
            let mut t = eth_common.clone();
            t[0].address = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831".into(); // USDC Arb
            t[1].address = "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9".into(); // USDT Arb
            t[2].address = "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1".into(); // DAI Arb
            t[3].address = "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1".into(); // WETH Arb
            t[4].address = "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f".into(); // WBTC Arb
            t
        }
        "eip155:10" => {
            let mut t = eth_common.clone();
            t[0].address = "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85".into(); // USDC OP
            t[1].address = "0x94b008aA00579c1307B0EF2c499aD98a8ce58e58".into(); // USDT OP
            t[2].address = "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1".into(); // DAI OP
            t[3].address = "0x4200000000000000000000000000000000000006".into(); // WETH OP
            t
        }
        _ => eth_common,
    }
}

fn build_balance_of_data(owner: &str) -> String {
    let selector = "70a08231";
    let addr = owner.trim_start_matches("0x");
    format!("0x{}{:0>24}{}", selector, "", addr)
}

fn resolve_rpc(chain_id: &str) -> &str {
    match chain_id {
        "eip155:8453" => "https://mainnet.base.org",
        "eip155:137" => "https://polygon-rpc.com",
        "eip155:42161" => "https://arb1.arbitrum.io/rpc",
        "eip155:10" => "https://mainnet.optimism.io",
        _ => "https://eth.llamarpc.com",
    }
}

pub struct TokenDiscoveryService;

impl TokenDiscoveryService {
    pub fn new() -> Self {
        Self
    }

    pub async fn discover(&self, chain_id: &str, owner_address: &str) -> Result<Vec<TokenAsset>> {
        let tokens = known_tokens_for_chain(chain_id);
        if tokens.is_empty() {
            return Ok(vec![]);
        }

        let rpc_url = resolve_rpc(chain_id);
        let client = EvmRpcClient::new("evm", rpc_url)
            .map_err(|e| GradienceError::Http(format!("rpc client: {}", e)))?;

        let data = build_balance_of_data(owner_address);

        let mut assets = Vec::new();
        for token in tokens {
            let result = client.eth_call(&token.address, &data).await;
            let balance_hex = match result {
                Ok(hex) => hex,
                Err(_) => continue,
            };
            let balance_raw = u128::from_str_radix(balance_hex.trim_start_matches("0x"), 16)
                .unwrap_or(0);
            if balance_raw == 0 {
                continue;
            }
            let div = 10u64.pow(token.decimals as u32) as f64;
            let formatted = (balance_raw as f64) / div;
            assets.push(TokenAsset {
                chain_id: chain_id.into(),
                address: owner_address.into(),
                token_address: token.address,
                symbol: token.symbol,
                name: token.name,
                decimals: token.decimals,
                balance: format!("0x{:x}", balance_raw),
                balance_formatted: format!("{:.6}", formatted).trim_end_matches('0').trim_end_matches('.').to_string(),
            });
        }
        Ok(assets)
    }
}
