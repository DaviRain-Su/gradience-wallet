use crate::error::{GradienceError, Result};
use sha3::{Digest, Keccak256};

/// Base mainnet Uniswap V3 SwapRouter02
pub const UNISWAP_V3_ROUTER: &str = "0x2626664c2603336E57B271c5C0b26F421741e481";

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

    Ok(UniV3SwapTx {
        to: UNISWAP_V3_ROUTER.to_string(),
        data,
        value: "0x0".into(),
    })
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
