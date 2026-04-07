use crate::context::AppContext;
use anyhow::Result;

pub async fn quote(_ctx: &AppContext, wallet_id: String, from: String, to: String, amount: String) -> Result<()> {
    let svc = gradience_core::dex::service::DexService::new();
    let q = svc.get_quote(&wallet_id, &from, &to, &amount).await?;
    println!("Quote from {}: swap {} {} -> {} {}
      Price impact: {}", q.provider, q.from_amount, q.from_token, q.to_amount, q.to_token, q.price_impact);
    Ok(())
}

pub async fn swap(_ctx: &AppContext, wallet_id: String, from: String, to: String, amount: String) -> Result<()> {
    let svc = gradience_core::dex::service::DexService::new();
    let tx = svc.execute_swap(&wallet_id, &from, &to, &amount).await?;
    println!("Swap submitted. Mock tx hash: {}", tx);
    Ok(())
}
