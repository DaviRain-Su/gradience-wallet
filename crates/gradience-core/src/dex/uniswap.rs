use crate::error::{GradienceError, Result};
use sha3::{Digest, Keccak256};

pub struct RouterConfig {
    pub router: String,
    pub quoter: Option<String>,
}

pub fn router_for_chain(chain_num: u64) -> RouterConfig {
    match chain_num {
        8453 => RouterConfig {
            // Base mainnet Uniswap V3 SwapRouter02
            router: "0x2626664c2603336E57B271c5C0b26F421741e481".into(),
            quoter: Some("0x3d4e44Eb1374040B13B768b5b1BD4a6F7B10bA7A".into()),
        },
        1 => RouterConfig {
            // Ethereum mainnet
            router: "0xE592427A768AE2aFed62EE5353cC928768B56B74".into(),
            quoter: Some("0x61fFE014bA17989E743c5F6cB21bF969dc0b63e4".into()),
        },
        42161 => RouterConfig {
            // Arbitrum
            router: "0xE592427A768AE2aFed62EE5353cC928768B56B74".into(),
            quoter: Some("0x61fFE014bA17989E743c5F6cB21bF969dc0b63e4".into()),
        },
        10 => RouterConfig {
            // Optimism
            router: "0xE592427A768AE2aFed62EE5353cC928768B56B74".into(),
            quoter: Some("0x61fFE014bA17989E743c5F6cB21bF969dc0b63e4".into()),
        },
        56 => RouterConfig {
            // BSC - PancakeSwap V3 Router
            router: "0x13f4EA83E0d3b33B7EcFcA1D8F6b4cD6c".into(),
            quoter: None,
        },
        _ => RouterConfig {
            router: "0x2626664c2603336E57B271c5C0b26F421741e481".into(),
            quoter: None,
        },
    }
}

pub struct UniV3SwapTx {
    pub to: String,
    pub data: Vec<u8>,
    pub value: String,
}

/// Encode `exactInputSingle((address,address,uint24,address,uint256,uint256,uint160))`
/// Params: tokenIn, tokenOut, fee, recipient, amountIn, amountOutMinimum, sqrtPriceLimitX96
pub fn encode_exact_input_single(
    token_in: &str,
    token_out: &str,
    fee: u24,
    recipient: &str,
    amount_in: &str,
    amount_out_min: &str,
    sqrt_price_limit_x96: &str,
    chain_num: u64,
) -> Result<UniV3SwapTx> {
    let selector = &Keccak256::digest(b"exactInputSingle((address,address,uint24,address,uint256,uint256,uint160)")[..4];

    let token_in = parse_address(token_in)?;
    let token_out = parse_address(token_out)?;
    let recipient = parse_address(recipient)?;
    let amount_in = u128::from_str_radix(amount_in.trim_start_matches("0x"), 16)
        .map_err(|_| GradienceError::Validation(format!("bad amount_in: {}", amount_in)))?;
    let amount_out_min = u128::from_str_radix(amount_out_min.trim_start_matches("0x"), 16)
        .map_err(|_| GradienceError::Validation(format!("bad amount_out_min: {}", amount_out_min)))?;
    let sqrt_price_limit = u128::from_str_radix(sqrt_price_limit_x96.trim_start_matches("0x"), 16)
        .unwrap_or(0);

    let mut data = selector.to_vec();
    // tuple is dynamic? No, exactInputSingle takes a single tuple which is static (all fixed size).
    // But in ABI encoding, a tuple of static types is treated like a static struct: no offset header.
    // However because the function takes a single tuple param, the encoding starts with the tuple contents directly after selector.
    data.extend(pad_address(&token_in));
    data.extend(pad_address(&token_out));
    data.extend(pad_u24(fee));
    data.extend(pad_address(&recipient));
    data.extend(pad_u256(amount_in));
    data.extend(pad_u256(amount_out_min));
    data.extend(pad_u256(sqrt_price_limit));

    let cfg = router_for_chain(chain_num);
    Ok(UniV3SwapTx {
        to: cfg.router,
        data,
        value: "0x0".into(),
    })
}

/// Encode QuoterV2 `quoteExactInputSingle(address,address,uint24,uint256,uint160)` call.
/// Selector: 0xcdca1753
pub fn encode_quote_exact_input_single(
    token_in: &str,
    token_out: &str,
    fee: u24,
    amount_in: u128,
    sqrt_price_limit_x96: u128,
) -> Result<Vec<u8>> {
    let selector = hex::decode("cdca1753").map_err(|e| GradienceError::Validation(format!("bad selector: {}", e)))?;
    let token_in = parse_address(token_in)?;
    let token_out = parse_address(token_out)?;
    let mut data = selector;
    data.extend(pad_address(&token_in));
    data.extend(pad_address(&token_out));
    data.extend(pad_u24(fee));
    data.extend(pad_u256(amount_in));
    data.extend(pad_u256(sqrt_price_limit_x96));
    Ok(data)
}

fn parse_address(addr: &str) -> Result<[u8; 20]> {
    let hex = addr.trim_start_matches("0x");
    let bytes = hex::decode(hex).map_err(|e| GradienceError::Validation(format!("invalid address: {}", e)))?;
    if bytes.len() != 20 {
        return Err(GradienceError::Validation("address length invalid".into()));
    }
    let mut arr = [0u8; 20];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn pad_address(addr: &[u8; 20]) -> Vec<u8> {
    let mut out = vec![0u8; 12];
    out.extend_from_slice(addr);
    out
}

fn pad_u24(v: u24) -> Vec<u8> {
    let mut out = vec![0u8; 29];
    out.extend_from_slice(&v.to_be_bytes());
    out
}

fn pad_u256(v: u128) -> Vec<u8> {
    let mut out = vec![0u8; 16];
    out.extend_from_slice(&v.to_be_bytes());
    out
}

// tiny u24 type alias using u32 with validation
pub type u24 = u32;
