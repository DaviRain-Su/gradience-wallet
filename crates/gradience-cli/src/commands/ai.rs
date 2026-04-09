use crate::context::AppContext;
use anyhow::Result;

pub async fn balance(ctx: &AppContext, wallet_id: String) -> Result<()> {
    let svc = gradience_core::ai::gateway::AiGatewayService::new();
    let bal = svc.get_balance(&ctx.db, &wallet_id, "USDC").await?;
    println!("AI Balance for {}: {} USDC (raw)", wallet_id, bal);
    Ok(())
}

pub async fn generate(ctx: &AppContext, wallet_id: String, prompt: String) -> Result<()> {
    let svc = gradience_core::ai::gateway::AiGatewayService::new();
    let resp = svc
        .llm_generate(
            &ctx.db,
            &wallet_id,
            None,
            "anthropic",
            "claude-3-5-sonnet",
            &prompt,
        )
        .await?;
    println!("Generated:\n{}", resp.content);
    println!(
        "Cost: {} | Tokens: {} in / {} out",
        resp.cost_raw, resp.input_tokens, resp.output_tokens
    );
    Ok(())
}
