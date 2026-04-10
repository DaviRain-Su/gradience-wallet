use crate::context::AppContext;
use anyhow::Result;

pub async fn serve() -> Result<()> {
    gradience_mcp::server::run_stdio_server()?;
    Ok(())
}

pub async fn sign_tx(
    _ctx: &AppContext,
    wallet_id: String,
    chain_id: String,
    to: String,
    amount: String,
) -> Result<()> {
    let args = gradience_mcp::args::SignTxArgs {
        wallet_id,
        chain_id,
        transaction: gradience_mcp::args::TxBody {
            to,
            value: amount,
            data: Some("0x".into()),
        },
        approval_id: None,
        session_id: None,
    };
    let resp = gradience_mcp::tools::handle_sign_transaction(args)?;
    println!("{}", resp);
    Ok(())
}

pub async fn balance(_ctx: &AppContext, wallet_id: String, chain_id: String) -> Result<()> {
    let args = gradience_mcp::args::GetBalanceArgs {
        wallet_id,
        chain_id: Some(chain_id),
    };
    let resp = gradience_mcp::tools::handle_get_balance(args)?;
    println!("{}", resp);
    Ok(())
}
