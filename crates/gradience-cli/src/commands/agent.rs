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

    // Demo: owner_id fixed to "user-1" for development simplicity
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
            || (chain == "ethereum" && a.chain_id == "eip155:1")
            || (chain == "conflux" && (a.chain_id == "eip155:1030" || a.chain_id == "eip155:71" || is_evm))
            || (chain == "conflux-core" && a.chain_id.starts_with("cfx:"))
            || (chain == "solana" && a.chain_id.starts_with("solana:"))
            || (chain == "ton" && a.chain_id.starts_with("ton:"));
        if is_match {
            found = true;
            if chain == "solana" || a.chain_id.starts_with("solana:") {
                let rpc_url = "https://api.devnet.solana.com";
                let client = gradience_core::rpc::solana::SolanaRpcClient::new(rpc_url);
                match client.get_balance(&a.address).await {
                    Ok(lamports) => {
                        let sol = gradience_core::rpc::solana::lamports_to_sol(lamports);
                        println!("Wallet {} on {}: {} SOL ({} lamports) (address: {})", wallet_id, a.chain_id, sol, lamports, a.address);
                    }
                    Err(e) => println!("Failed to get balance for {}: {}", a.address, e),
                }
            } else if chain == "ton" || a.chain_id.starts_with("ton:") {
                let rpc_url = gradience_core::chain::resolve_rpc(&a.chain_id);
                let client = gradience_core::rpc::ton::TonRpcClient::new_with_url(rpc_url);
                match client.get_balance(&a.address).await {
                    Ok(nanoton) => {
                        let ton = nanoton as f64 / 1e9;
                        println!("Wallet {} on {}: {} TON ({} nanoton) (address: {})", wallet_id, a.chain_id, ton, nanoton, a.address);
                    }
                    Err(e) => println!("Failed to get balance for {}: {}", a.address, e),
                }
            } else if chain == "conflux-core" || a.chain_id.starts_with("cfx:") {
                let rpc_url = gradience_core::chain::resolve_rpc(&a.chain_id);
                let client = gradience_core::rpc::conflux_core::ConfluxCoreRpcClient::new_with_url(rpc_url);
                match client.get_balance(&a.address).await {
                    Ok(drip) => {
                        let cfx = drip as f64 / 1e18;
                        println!("Wallet {} on {}: {} CFX ({} drip) (address: {})", wallet_id, a.chain_id, cfx, drip, a.address);
                    }
                    Err(e) => println!("Failed to get balance for {}: {}", a.address, e),
                }
            } else {
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

    // ------------------------------------------------------------------
    // Solana branch
    // ------------------------------------------------------------------
    if chain == "solana" {
        let mut sol_addr = None;
        for a in &addrs {
            if a.chain_id.starts_with("solana:") {
                sol_addr = Some(a.address.clone());
                break;
            }
        }
        let from_addr = sol_addr.ok_or_else(|| anyhow::anyhow!("No Solana address found for wallet {}", wallet_id))?;
        let to_addr = to.unwrap_or_else(|| from_addr.clone());

        let sol_amount: f64 = amount.parse()
            .map_err(|_| anyhow::anyhow!("Invalid amount: expected SOL decimal string"))?;
        let lamports = (sol_amount * 1_000_000_000.0) as u64;

        let rpc_url = "https://api.devnet.solana.com";
        let client = gradience_core::rpc::solana::SolanaRpcClient::new(rpc_url);
        let blockhash = client.get_latest_blockhash().await?;

        let tx_bytes = gradience_core::ows::signing::build_solana_transfer_tx(
            &from_addr,
            &to_addr,
            lamports,
            &blockhash,
        ).map_err(|e| anyhow::anyhow!("Failed to build Solana tx: {}", e))?;
        let tx_hex = format!("0x{}", hex::encode(&tx_bytes));

        let result = ows_lib::sign_and_send(
            &wallet_id,
            "solana",
            &tx_hex,
            Some(&passphrase),
            None,
            Some(rpc_url),
            Some(&ctx.vault_dir),
        ).map_err(|e| anyhow::anyhow!("OWS sign_and_send failed: {}", e))?;

        println!(
            "Sent {} SOL from {} to {} on Solana devnet. Signature: {}",
            amount, from_addr, to_addr, result.tx_hash
        );
        return Ok(());
    }

    // ------------------------------------------------------------------
    // TON branch
    // ------------------------------------------------------------------
    if chain == "ton" || chain == "toncoin" {
        let mut ton_addr = None;
        for a in &addrs {
            if a.chain_id.starts_with("ton:") {
                ton_addr = Some(a.address.clone());
                break;
            }
        }
        let from_addr = ton_addr.ok_or_else(|| anyhow::anyhow!("No TON address found for wallet {}", wallet_id))?;
        let to_addr = to.unwrap_or_else(|| from_addr.clone());

        let ton_amount: f64 = amount.parse()
            .map_err(|_| anyhow::anyhow!("Invalid amount: expected TON decimal string"))?;
        let nanoton = (ton_amount * 1_000_000_000.0) as u64;

        let rpc_url = gradience_core::chain::resolve_rpc(&chain);
        let client = gradience_core::rpc::ton::TonRpcClient::new_with_url(rpc_url);
        let seqno = client.get_seqno(&from_addr).await?;

        let vault = ctx.ows.init_vault(&passphrase).await?;
        let tx = gradience_core::ows::adapter::Transaction {
            to: Some(to_addr.clone()),
            value: nanoton.to_string(),
            data: seqno.to_be_bytes().to_vec(),
            raw_hex: "".into(),
        };
        let signed = ctx.ows.sign_transaction(&vault, &wallet_id, "ton:0", &tx, &passphrase
        ).await.map_err(|e| anyhow::anyhow!("TON sign_transaction failed: {}", e))?;
        let result = ctx.ows.broadcast("ton:0", &signed, rpc_url).await.map_err(|e| anyhow::anyhow!("TON broadcast failed: {}", e))?;

        println!(
            "Sent {} TON from {} to {} on TON testnet. Result: {}",
            amount, from_addr, to_addr, result
        );
        return Ok(());
    }

    // ------------------------------------------------------------------
    // Conflux Core Space branch
    // ------------------------------------------------------------------
    if chain == "conflux-core" || chain.starts_with("cfx:") {
        let mut cfx_addr = None;
        for a in &addrs {
            if a.chain_id.starts_with("cfx:") {
                cfx_addr = Some(a.address.clone());
                break;
            }
        }
        let from_addr = cfx_addr.ok_or_else(|| anyhow::anyhow!("No Conflux Core address found for wallet {}", wallet_id))?;
        let to_addr = to.unwrap_or_else(|| from_addr.clone());

        let amount_cfx: f64 = amount.parse()
            .map_err(|_| anyhow::anyhow!("Invalid amount: expected CFX decimal string"))?;
        let drip = (amount_cfx * 1_000_000_000_000_000_000.0) as u128;
        let value_hex = format!("0x{:x}", drip);

        let vault = ctx.ows.init_vault(&passphrase).await?;
        let tx = gradience_core::ows::adapter::Transaction {
            to: Some(to_addr.clone()),
            value: value_hex,
            data: vec![],
            raw_hex: "".into(),
        };
        let signed = ctx.ows.sign_transaction(&vault, &wallet_id, "cfx:1", &tx, &passphrase
        ).await.map_err(|e| anyhow::anyhow!("Conflux Core sign_transaction failed: {}", e))?;
        let result = ctx.ows.broadcast("cfx:1", &signed, "").await
            .map_err(|e| anyhow::anyhow!("Conflux Core broadcast failed: {}", e))?;

        println!(
            "Sent {} CFX from {} to {} on Conflux Core testnet. Result: {}",
            amount, from_addr, to_addr, result
        );
        return Ok(());
    }

    // ------------------------------------------------------------------
    // EVM branch (original)
    // ------------------------------------------------------------------
    let mut addr = None;
    for a in &addrs {
        if gradience_core::chain::is_evm_chain(&a.chain_id) {
            addr = Some(a.address.clone());
            break;
        }
    }
    let from_addr = addr.ok_or_else(|| anyhow::anyhow!("No EVM address found for wallet {}", wallet_id))?;
    let to_addr = to.unwrap_or_else(|| from_addr.clone());

    let wei = gradience_core::eth_to_wei(&amount)
        .map_err(|_| anyhow::anyhow!("Invalid amount"))?;

    let rpc_url = gradience_core::chain::resolve_rpc(&chain);

    let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", rpc_url)?;
    let nonce = client.get_transaction_count(&from_addr).await?;
    let gas_price_hex = client.get_gas_price().await?;
    let gas_price = u128::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16)
        .map_err(|e| anyhow::anyhow!("Invalid gas price: {}", e))?;

    let chain_num = gradience_core::chain::evm_chain_num(&chain);

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
    let tx_hex = format!("0x{}", hex::encode(rlp.out()));

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
