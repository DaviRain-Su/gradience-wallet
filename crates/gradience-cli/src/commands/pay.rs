use crate::context::AppContext;
use anyhow::Result;
use gradience_db::queries;
use gradience_core::payment::mpp_client::{GradienceMppProvider, MppClient};
use gradience_core::payment::router::PaymentRouter;

pub async fn mpp_pay(
    ctx: &AppContext,
    wallet_id: String,
    recipient: String,
    _amount: String,
    _token: String,
    _chain: Option<String>,
    _deadline: Option<u64>,
) -> Result<()> {
    let wallet = queries::get_wallet_by_id(&ctx.db, &wallet_id).await?;
    if wallet.is_none() {
        anyhow::bail!("Wallet not found: {}", wallet_id);
    }

    if !recipient.starts_with("http://") && !recipient.starts_with("https://") {
        anyhow::bail!("MPP pay currently requires an HTTP(S) endpoint URL as recipient.");
    }

    let router = PaymentRouter::default();
    let provider = GradienceMppProvider::new(&wallet_id, router);
    let client = MppClient::new(provider);

    let req = client.http.get(&recipient);
    let resp = client
        .send(req)
        .await
        .map_err(|e| anyhow::anyhow!("MPP payment request failed: {}", e))?;

    println!("MPP request completed. Status: {}", resp.status());
    let body = resp.text().await.unwrap_or_default();
    println!("Body: {}", body);
    Ok(())
}
