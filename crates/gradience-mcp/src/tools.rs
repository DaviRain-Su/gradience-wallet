use gradience_core::ows::adapter::Transaction;
use gradience_core::policy::engine::{PolicyEngine, EvalContext, Decision, Policy};
use serde_json::json;

pub fn handle_sign_transaction(params: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let wallet_id = params.get("walletId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing walletId"))?;
    let chain_id = params.get("chainId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing chainId"))?;
    let tx_obj = params.get("transaction")
        .ok_or_else(|| anyhow::anyhow!("missing transaction"))?;
    
    let to = tx_obj.get("to").and_then(|v| v.as_str()).map(|s| s.to_string());
    let value = tx_obj.get("value").and_then(|v| v.as_str()).unwrap_or("0").to_string();
    let data_hex = tx_obj.get("data").and_then(|v| v.as_str()).unwrap_or("0x");
    let data = hex::decode(data_hex.trim_start_matches("0x")).unwrap_or_default();
    
    let tx = Transaction { to, value: value.clone(), data, raw_hex: data_hex.into() };
    let engine = PolicyEngine;
    let ctx = EvalContext {
        wallet_id: wallet_id.into(),
        api_key_id: "mcp-key".into(),
        chain_id: chain_id.into(),
        transaction: tx,
        intent: None,
        timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
    };
    
    // Demo: always allow base chain, deny invalid chains
    let base_policy = Policy {
        id: "demo".into(),
        name: "demo".into(),
        wallet_id: None,
        workspace_id: None,
        rules: vec![gradience_core::policy::engine::Rule::ChainWhitelist {
            chain_ids: vec!["eip155:8453".into()],
        }],
        priority: 1,
        status: "active".into(),
        version: 1,
        created_at: "".into(),
        updated_at: "".into(),
    };
    
    let result = engine.evaluate(ctx, vec![&base_policy])?;
    
    match result.decision {
        Decision::Allow => {
            Ok(json!({
                "signature": format!("0x{}_sig", wallet_id),
                "decision": "allowed",
                "chainId": chain_id,
            }))
        }
        Decision::Deny => {
            Err(anyhow::anyhow!("POLICY_DENIED: {}", result.reasons.join(", ")))
        }
        Decision::Warn => {
            Ok(json!({
                "signature": null,
                "decision": "warned",
                "reasons": result.reasons,
            }))
        }
    }
}

pub fn handle_get_balance(params: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let wallet_id = params.get("walletId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing walletId"))?;
    let chain_id = params.get("chainId")
        .and_then(|v| v.as_str())
        .unwrap_or("eip155:8453");
    
    Ok(json!({
        "walletId": wallet_id,
        "chainId": chain_id,
        "native": {
            "symbol": "ETH",
            "balance": "1.2345",
            "decimals": 18,
        },
        "tokens": [
            {
                "symbol": "USDC",
                "address": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
                "balance": "500.0",
                "decimals": 6,
            }
        ]
    }))
}
