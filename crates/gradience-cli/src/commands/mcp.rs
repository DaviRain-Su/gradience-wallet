use crate::context::AppContext;
use anyhow::Result;
use serde_json::json;

pub async fn serve() -> Result<()> {
    gradience_mcp::server::run_stdio_server()?;
    Ok(())
}

pub async fn sign_tx(_ctx: &AppContext, wallet_id: String, chain_id: String, to: String, amount: String) -> Result<()> {
    let resp = gradience_mcp::tools::handle_sign_transaction(json!({
        "walletId": wallet_id,
        "chainId": chain_id,
        "transaction": {
            "to": to,
            "value": amount,
            "data": "0x"
        }
    }))?;
    println!("{}", resp.to_string());
    Ok(())
}

pub async fn balance(_ctx: &AppContext, wallet_id: String, chain_id: String) -> Result<()> {
    let resp = gradience_mcp::tools::handle_get_balance(json!({
        "walletId": wallet_id,
        "chainId": chain_id
    }))?;
    println!("{}", resp.to_string());
    Ok(())
}
