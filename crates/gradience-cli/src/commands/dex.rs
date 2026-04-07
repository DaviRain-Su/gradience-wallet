use crate::context::AppContext;
use anyhow::Result;

pub async fn quote(_ctx: &AppContext, wallet_id: String, from: String, to: String, amount: String) -> Result<()> {
    let svc = gradience_core::dex::service::DexService::new();
    let q = svc.get_quote(&wallet_id, &from, &to, &amount, 8453).await?;
    println!("Quote from {}: swap {} {} -> {} {}
      Price impact: {}", q.provider, q.from_amount, q.from_token, q.to_amount, q.to_token, q.price_impact);
    Ok(())
}

pub async fn swap(ctx: &AppContext, wallet_id: String, from: String, to: String, amount: String) -> Result<()> {
    let passphrase = ctx.read_passphrase().ok_or_else(|| anyhow::anyhow!("passphrase not found. Run unlock first."))?;
    let rpc_url = "https://mainnet.base.org";
    let chain_num = 8453u64;
    let chain = "base";

    let addrs = gradience_db::queries::list_wallet_addresses(&ctx.db, &wallet_id).await?;
    let mut from_addr = None;
    for a in &addrs {
        if a.chain_id == "eip155:8453" || a.chain_id == "eip155:1" {
            from_addr = Some(a.address.clone());
            break;
        }
    }
    let from_addr = from_addr.ok_or_else(|| anyhow::anyhow!("wallet address not found"))?;

    let dex = gradience_core::dex::service::DexService::new();
    let tx = dex.build_swap_tx(&from_addr, &from, &to, &amount, chain_num).await?;

    let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", rpc_url)?;
    let nonce = client.get_transaction_count(&from_addr).await?;
    let gp_hex = client.get_gas_price().await?;
    let gas_price = u128::from_str_radix(gp_hex.trim_start_matches("0x"), 16)?;

    let to_bytes = hex::decode(tx.to.as_deref().unwrap_or("").trim_start_matches("0x")).unwrap_or_default();
    let data = tx.data;
    let value = tx.value.parse::<u128>().unwrap_or(0);

    let mut rlp = rlp::RlpStream::new_list(9);
    rlp.append(&nonce);
    rlp.append(&gas_price);
    rlp.append(&300000u64);
    rlp.append(&to_bytes);
    rlp.append(&value);
    rlp.append(&data);
    rlp.append(&chain_num);
    rlp.append(&0u8);
    rlp.append(&0u8);
    let tx_hex = format!("0x{}", hex::encode(&rlp.out()));

    let result = ows_lib::sign_and_send(
        &wallet_id,
        chain,
        &tx_hex,
        Some(&passphrase),
        None,
        Some(rpc_url),
        Some(&ctx.vault_dir),
    )?;

    println!("Swap submitted. Tx hash: {}", result.tx_hash);
    Ok(())
}
