use crate::ai::balance::AiBalanceService;
use anyhow::Result;
use sqlx::{Pool, Sqlite};
use std::time::Instant;

pub struct AiGatewayService {
    balance: AiBalanceService,
}

impl Default for AiGatewayService {
    fn default() -> Self {
        Self::new()
    }
}

impl AiGatewayService {
    pub fn new() -> Self {
        Self {
            balance: AiBalanceService::new(),
        }
    }

    /// Pre-deduct estimated cost, run mock LLM, then reconcile actual cost.
    pub async fn llm_generate(
        &self,
        pool: &Pool<Sqlite>,
        wallet_id: &str,
        api_key_id: Option<&str>,
        provider: &str,
        model: &str,
        prompt: &str,
    ) -> Result<LlmResponse> {
        gradience_db::queries::seed_model_pricing(pool).await.ok(); // ensure seed exists

        let pricing = gradience_db::queries::get_model_pricing(pool, provider, model).await?;
        let pricing = pricing.ok_or_else(|| anyhow::anyhow!("No pricing found for {}/{}", provider, model))?;

        // Estimate tokens (very rough: 1 token ≈ 4 chars)
        let input_chars = prompt.len() as i64;
        let estimated_input_tokens = (input_chars / 4).max(1);
        let estimated_output_tokens = 100i64; // assume 100 tokens output

        // Cost in raw USDC wei-like units (1 USDC = 1_000_000)
        // price is per 1M tokens
        let scale = 1_000_000i64;
        let est_cost = (estimated_input_tokens * pricing.input_per_m + estimated_output_tokens * pricing.output_per_m) / 1_000_000;
        let est_cost_raw = (est_cost * scale).to_string();

        // Pre-deduct
        let ok = self.balance.deduct(pool, wallet_id, &pricing.currency, &est_cost_raw).await?;
        if !ok {
            return Ok(LlmResponse {
                content: "Insufficient AI balance.".into(),
                input_tokens: estimated_input_tokens,
                output_tokens: 0,
                cost_raw: "0".into(),
                status: "insufficient_balance".into(),
            });
        }

        let start = Instant::now();

        // Try real API, fallback to mock
        let (output_text, actual_input_tokens, actual_output_tokens, status_str) =
            if provider.eq_ignore_ascii_case("anthropic") {
                if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
                    match super::providers::anthropic::call_anthropic(&key, model, prompt).await {
                        Ok(res) => {
                            (
                                res.content,
                                res.input_tokens,
                                res.output_tokens,
                                "success"
                            )
                        }
                        Err(e) => {
                            tracing::warn!("Anthropic call failed: {}", e);
                            let fallback = format!(
                                "[Anthropic unavailable] This is a mock response from {} {} for prompt length {} chars.",
                                provider, model, input_chars
                            );
                            (fallback, estimated_input_tokens, 25i64, "fallback_mock")
                        }
                    }
                } else {
                    let fallback = format!(
                        "[ANTHROPIC_API_KEY not set] This is a mock response from {} {} for prompt length {} chars.",
                        provider, model, input_chars
                    );
                    (fallback, estimated_input_tokens, 25i64, "fallback_mock")
                }
            } else {
                let fallback = format!(
                    "This is a mock response from {} {} for prompt length {} chars.",
                    provider, model, input_chars
                );
                (fallback, estimated_input_tokens, 25i64, "fallback_mock")
            };

        let duration_ms = start.elapsed().as_millis() as i32;

        let actual_cost = (actual_input_tokens * pricing.input_per_m + actual_output_tokens * pricing.output_per_m) / 1_000_000;
        let actual_cost_raw = (actual_cost * scale).to_string();

        // Refund over-estimation
        if actual_cost_raw != est_cost_raw {
            let est: i64 = est_cost_raw.parse().unwrap_or(0);
            let act: i64 = actual_cost_raw.parse().unwrap_or(0);
            if est > act {
                let _ = self.balance.topup(pool, wallet_id, &pricing.currency, &(est - act).to_string()).await;
            }
        } else if actual_cost_raw.parse::<i64>().unwrap_or(0) > est_cost_raw.parse::<i64>().unwrap_or(0) {
            // Under-estimation: deduct additional
            let est: i64 = est_cost_raw.parse().unwrap_or(0);
            let act: i64 = actual_cost_raw.parse().unwrap_or(0);
            let _ = self.balance.deduct(pool, wallet_id, &pricing.currency, &(act - est).to_string()).await;
        }

        // Log
        gradience_db::queries::insert_llm_call_log(
            pool,
            wallet_id,
            api_key_id,
            provider,
            model,
            actual_input_tokens,
            actual_output_tokens,
            None,
            &actual_cost_raw,
            duration_ms,
            status_str,
        )
        .await?;

        Ok(LlmResponse {
            content: output_text,
            input_tokens: actual_input_tokens,
            output_tokens: actual_output_tokens,
            cost_raw: actual_cost_raw,
            status: if status_str == "success" { "success".into() } else { status_str.into() },
        })
    }

    pub async fn get_balance(
        &self,
        pool: &Pool<Sqlite>,
        wallet_id: &str,
        token: &str,
    ) -> Result<String> {
        self.balance.get_balance(pool, wallet_id, token).await
    }

    pub async fn topup(
        &self,
        pool: &Pool<Sqlite>,
        wallet_id: &str,
        token: &str,
        amount_raw: &str,
    ) -> Result<()> {
        self.balance.topup(pool, wallet_id, token, amount_raw).await
    }
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_raw: String,
    pub status: String,
}
