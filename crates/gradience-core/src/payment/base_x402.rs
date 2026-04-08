use crate::error::{GradienceError, Result};
use reqwest::Method;
use std::collections::HashMap;

/// Client for Base/EVM x402 payments via the Node.js bridge.
/// The bridge uses @x402/evm + @x402/fetch to automatically handle
/// EIP-3009 TransferWithAuthorization signing and retry.
pub struct BaseX402Client {
    bridge_dir: std::path::PathBuf,
}

impl BaseX402Client {
    pub fn new(bridge_dir: std::path::PathBuf) -> Self {
        Self { bridge_dir }
    }

    /// Execute an x402 payment for the given HTTP request.
    /// The Node bridge wraps fetch with @x402/fetch, so 402 handling,
    /// signing, and retry all happen inside the bridge.
    ///
    /// Returns the final HTTP status, response headers, body text,
    /// and an optional on-chain tx hash extracted from payment-response.
    pub async fn pay(
        &self,
        private_key: &str,
        network: &str,
        method: Method,
        url: &str,
        headers: Vec<(String, String)>,
        body: Option<String>,
    ) -> Result<(u16, HashMap<String, String>, String, Option<String>)> {
        let input = serde_json::json!({
            "privateKey": private_key,
            "network": network,
            "url": url,
            "method": method.as_str(),
            "headers": headers.iter().cloned().collect::<HashMap<String, String>>(),
            "body": body,
        });

        let output = tokio::process::Command::new("node")
            .arg("index.mjs")
            .arg(input.to_string())
            .current_dir(&self.bridge_dir)
            .output()
            .await
            .map_err(|e| {
                GradienceError::Validation(format!(
                    "failed to spawn Base x402 bridge: {} (is Node.js installed?)",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GradienceError::Validation(format!(
                "Base x402 bridge failed: {}",
                stderr
            )));
        }

        let result: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| {
                GradienceError::Validation(format!(
                    "Base x402 bridge output parse error: {}",
                    e
                ))
            })?;

        if !result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            let err = result
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            return Err(GradienceError::Validation(format!(
                "Base x402 payment failed: {}",
                err
            )));
        }

        let status = result["status"].as_u64().unwrap_or(0) as u16;
        let resp_headers: HashMap<String, String> = result["headers"]
            .as_object()
            .map(|o| {
                o.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        let resp_body = result["body"].as_str().unwrap_or("").to_string();

        let tx_hash = resp_headers
            .get("payment-response")
            .or_else(|| resp_headers.get("Payment-Response"))
            .and_then(|s| {
                let decoded = base64::decode(s).ok()?;
                let json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
                json.get("transaction")
                    .and_then(|t| t.as_str())
                    .map(|t| t.to_string())
            });

        Ok((status, resp_headers, resp_body, tx_hash))
    }
}
