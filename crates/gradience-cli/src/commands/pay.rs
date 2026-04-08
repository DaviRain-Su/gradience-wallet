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

    if chain.starts_with("stellar") {
        let addrs = queries::list_wallet_addresses(&ctx.db, &wallet_id).await.unwrap_or_default();
        let stellar_addr = addrs
            .iter()
            .find(|a| a.chain_id.starts_with("stellar:"))
            .ok_or_else(|| anyhow::anyhow!("No Stellar address found for wallet {}", wallet_id))?
            .address
            .clone();

        let derivation_path = gradience_core::wallet::hd::path_for(&chain, 0);
        let seed = gradience_core::ows::local_adapter::derive_demo_seed(&wallet_id, &chain, &derivation_path);
        let secret_key = gradience_core::payment::stellar_x402::stellar_secret_from_seed(&seed);

        let bridge_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(|p| std::path::PathBuf::from(p).join("../../bridge/stellar-x402"))
            .unwrap_or_else(|_| std::path::PathBuf::from("./bridge/stellar-x402"));
        let x402_client = gradience_core::payment::stellar_x402::StellarX402Client::new(bridge_dir);
        let network = if chain.contains("testnet") { "stellar:testnet" } else { "stellar:pubnet" };
        let (resp, tx_hash) = x402_client
            .pay(
                &secret_key,
                network,
                reqwest::Method::GET,
                &recipient,
                vec![],
                None,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Stellar x402 payment failed: {}", e))?;

        println!(
            "Stellar x402 payment settled.\n  status: {}\n  tx_hash: {}\n  stellar_address: {}\n  body: {}",
            resp.status().as_u16(),
            tx_hash.unwrap_or_else(|| "N/A".into()),
            stellar_addr,
            resp.text().await.unwrap_or_default()
        );
        return Ok(());
    }

    if chain.starts_with("eip155") {
        let addrs = queries::list_wallet_addresses(&ctx.db, &wallet_id).await.unwrap_or_default();
        let evm_addr = addrs
            .iter()
            .find(|a| a.chain_id == chain)
            .or_else(|| addrs.iter().find(|a| a.chain_id == "eip155:8453"))
            .or_else(|| addrs.iter().find(|a| a.chain_id == "eip155:1"))
            .ok_or_else(|| anyhow::anyhow!("No EVM address found for wallet {}", wallet_id))?
            .address
            .clone();

        let derivation_path = gradience_core::wallet::hd::path_for(&chain, 0);
        let seed = gradience_core::ows::local_adapter::derive_demo_seed(&wallet_id, &chain, &derivation_path);
        let private_key = format!("0x{}", hex::encode(&seed));

        let bridge_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(|p| std::path::PathBuf::from(p).join("../../bridge/base-x402"))
            .unwrap_or_else(|_| std::path::PathBuf::from("./bridge/base-x402"));
        let x402_client = gradience_core::payment::base_x402::BaseX402Client::new(bridge_dir);
        let (status, _headers, body, tx_hash) = x402_client
            .pay(&private_key, &chain, reqwest::Method::GET, &recipient, vec![], None)
            .await
            .map_err(|e| anyhow::anyhow!("Base/EVM x402 payment failed: {}", e))?;

        println!(
            "Base/EVM x402 payment settled.\n  status: {}\n  tx_hash: {}\n  evmAddress: {}\n  body: {}",
            status,
            tx_hash.unwrap_or_else(|| "N/A".into()),
            evm_addr,
            body
        );
        return Ok(());
    }

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
    let req = svc.create_requirement(&recipient, &amount, &token, deadline, Some(&chain))?;
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
