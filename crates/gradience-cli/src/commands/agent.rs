use crate::context::AppContext;
use anyhow::Result;
use gradience_core::ows::adapter::OwsAdapter;
use gradience_db::queries;

pub async fn create(ctx: &AppContext, name: String, _workspace: Option<String>) -> Result<()> {
    if name.trim().is_empty() {
        anyhow::bail!("Wallet name cannot be empty");
    }

    let passphrase = ctx.read_passphrase()
        .ok_or_else(|| anyhow::anyhow!("No session found. Run 'gradience auth login' first."))?;

    // Demo: owner_id fixed to "user-1" for hackathon simplicity
    let owner_id = "user-1";
    if queries::get_user_by_email(&ctx.db, "demo@gradience.io").await?.is_none() {
        queries::create_user(&ctx.db, owner_id, "demo@gradience.io").await?;
    }

    let vault = ctx.ows.init_vault(&passphrase).await?;
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

pub async fn fund(
    ctx: &AppContext,
    wallet_id: String,
    amount: String,
    chain: Option<String>,
    to: Option<String>,
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
    let to_addr = to.unwrap_or_else(|| from_addr.clone());

    let wei = gradience_core::eth_to_wei(&amount)
        .map_err(|_| anyhow::anyhow!("Invalid amount"))?;

    let rpc_url = if chain == "base" {
        "https://mainnet.base.org"
    } else {
        "https://eth.llamarpc.com"
    };

    let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", rpc_url)?;
    let nonce = client.get_transaction_count(&from_addr).await?;
    let gas_price_hex = client.get_gas_price().await?;
    let gas_price = u128::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16)
        .map_err(|e| anyhow::anyhow!("Invalid gas price: {}", e))?;

    let chain_num = if chain == "base" { 8453u64 } else { 1u64 };

    let to_bytes = hex::decode(to_addr.trim_start_matches("0x")).unwrap_or_default();
    let mut rlp = rlp::RlpStream::new_list(9);
    rlp.append(&nonce);
    rlp.append(&gas_price);
    rlp.append(&21000u64);
    rlp.append(&to_bytes);
    rlp.append(&wei);
    rlp.append(&Vec::<u8>::new());
    rlp.append(&chain_num);
    rlp.append(&0u8);
    rlp.append(&0u8);
    let tx_hex = format!("0x{}", hex::encode(&rlp.out()));

    let result = ows_lib::sign_and_send(
        &wallet_id,
        &chain,
        &tx_hex,
        Some(&passphrase),
        None,
        Some(rpc_url),
        Some(&ctx.vault_dir),
    ).map_err(|e| anyhow::anyhow!("OWS sign_and_send failed: {}", e))?;

    println!(
        "Sent {} ETH from {} to {} on {}. Tx hash: {}",
        amount, from_addr, to_addr, chain, result.tx_hash
    );
    Ok(())
}
