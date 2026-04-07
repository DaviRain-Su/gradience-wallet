use crate::context::AppContext;
use anyhow::Result;
use gradience_core::ows::adapter::OwsAdapter;
use gradience_db::queries;

pub async fn create(ctx: &AppContext, name: String, _workspace: Option<String>) -> Result<()> {
    if name.trim().is_empty() {
        anyhow::bail!("Wallet name cannot be empty");
    }

    // Demo: owner_id fixed to "user-1" for hackathon simplicity
    let owner_id = "user-1";
    if queries::get_user_by_email(&ctx.db, "demo@gradience.io").await?.is_none() {
        queries::create_user(&ctx.db, owner_id, "demo@gradience.io").await?;
    }

    let vault = ctx.ows.init_vault("demo-pass-12345").await?;
    let wallet = ctx.ows.create_wallet(&vault, &name, Default::default()).await?;

    queries::create_wallet(&ctx.db, &wallet.id, &wallet.name, owner_id, None
    ).await?;

    for acc in &wallet.accounts {
        queries::create_wallet_address(
            &ctx.db,
            &uuid::Uuid::new_v4().to_string(),
            &wallet.id,
            &acc.chain_id,
            &acc.address,
            &acc.derivation_path,
        ).await?;
    }

    println!("Created wallet '{}' (id: {})", wallet.name, wallet.id);
    for acc in &wallet.accounts {
        println!("  [{}] {}", acc.chain_id, acc.address);
    }
    Ok(())
}

pub async fn list(ctx: &AppContext) -> Result<()> {
    let owner_id = "user-1";
    let wallets = queries::list_wallets_by_owner(&ctx.db, owner_id).await?;
    if wallets.is_empty() {
        println!("No wallets configured yet.");
        return Ok(());
    }
    for w in wallets {
        let addrs = queries::list_wallet_addresses(&ctx.db, &w.id).await.unwrap_or_default();
        println!("{} - {} ({} addresses)", w.id, w.name, addrs.len());
        for a in addrs {
            println!("  [{}] {}", a.chain_id, a.address);
        }
    }
    Ok(())
}

pub async fn balance(ctx: &AppContext, wallet_id: String, chain: Option<String>) -> Result<()> {
    let chain = chain.unwrap_or_else(|| "base".into());
    let wallet = queries::get_wallet_by_id(&ctx.db, &wallet_id).await?;
    if wallet.is_none() {
        anyhow::bail!("Wallet not found: {}", wallet_id);
    }
    let addrs = queries::list_wallet_addresses(&ctx.db, &wallet_id).await.unwrap_or_default();
    let mut found = false;
    for a in addrs {
        let is_evm = a.chain_id.starts_with("eip155:");
        let is_match = a.chain_id.contains(&chain)
            || (chain == "base" && (a.chain_id == "eip155:8453" || (is_evm && a.chain_id == "eip155:1")))
            || (chain == "ethereum" && a.chain_id == "eip155:1");
        if is_match {
            found = true;
            let rpc_url = if chain == "base" || a.chain_id == "eip155:8453" {
                "https://mainnet.base.org"
            } else {
                "https://eth.llamarpc.com"
            };
            let client = gradience_core::rpc::evm::EvmRpcClient::new(&a.chain_id, rpc_url)?;
            match client.get_balance(&a.address).await {
                Ok(bal) => println!("Wallet {} on {}: {} (address: {})", wallet_id, a.chain_id, bal, a.address),
                Err(e) => println!("Failed to get balance for {}: {}", a.address, e),
            }
        }
    }
    if !found {
        println!("No address found for chain {}", chain);
    }
    Ok(())
}

pub async fn fund(_ctx: &AppContext, wallet_id: String, amount: String, chain: Option<String>) -> Result<()> {
    let chain = chain.unwrap_or_else(|| "base".into());
    println!("Demo: Funded wallet {} with {} on chain {}", wallet_id, amount, chain);
    Ok(())
}
