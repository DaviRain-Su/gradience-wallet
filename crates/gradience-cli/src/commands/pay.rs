use crate::context::AppContext;
use anyhow::Result;
use gradience_db::queries;

pub async fn x402(
    ctx: &AppContext,
    wallet_id: String,
    recipient: String,
    amount: String,
    token: String,
    chain: Option<String>,
    deadline: Option<u64>,
) -> Result<()> {
    let chain = chain.unwrap_or_else(|| "base".into());
    let wallet = queries::get_wallet_by_id(&ctx.db, &wallet_id).await?;
    if wallet.is_none() {
        anyhow::bail!("Wallet not found: {}", wallet_id);
    }

    let passphrase = ctx.read_passphrase()
        .ok_or_else(|| anyhow::anyhow!("No session found. Run 'gradience auth login' first."))?;

    let addrs = queries::list_wallet_addresses(&ctx.db, &wallet_id).await.unwrap_or_default();
    let mut addr = None;
    for a in &addrs {
        if a.chain_id == "eip155:8453" || a.chain_id == "eip155:1" {
            addr = Some(a.address.clone());
            break;
        }
    }
    let from_addr = addr.ok_or_else(|| anyhow::anyhow!("No EVM address found for wallet {}", wallet_id))?;

    let deadline = deadline.unwrap_or_else(|| {
        (std::time::SystemTime::now() + std::time::Duration::from_secs(3600))
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    });

    let svc = gradience_core::payment::x402::X402Service::new();
    let req = svc.create_requirement(&recipient, &amount, &token, deadline)?;
    let sig = "dummy-signature-for-demo"; // In production this would be a real EIP-191/712 signature
    let mut payment = svc.sign_payment(req, sig)?;

    let tx_hash = svc.settle_payment(
        &mut payment,
        &wallet_id,
        &from_addr,
        &chain,
        &passphrase,
        &ctx.vault_dir,
    ).await?;

    println!("x402 payment settled. Tx hash: {}", tx_hash);
    Ok(())
}
