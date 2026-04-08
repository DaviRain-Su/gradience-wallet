use crate::error::{GradienceError, Result};
use reqwest::Method;
use serde_json::json;
use std::collections::HashMap;

/// Client for Stellar x402 payments via the Node.js bridge.
/// The bridge uses the official @x402/stellar library to handle
/// auth-entry signing and facilitator communication.
pub struct StellarX402Client {
    bridge_dir: std::path::PathBuf,
}

impl StellarX402Client {
    pub fn new(bridge_dir: std::path::PathBuf) -> Self {
        Self { bridge_dir }
    }

    /// Execute an x402 payment for the given HTTP request.
    /// If the server responds with 402, the bridge is invoked to sign the
    /// Soroban authorization entries, and the request is retried.
    ///
    /// Returns the final HTTP response and an optional on-chain tx hash.
    pub async fn pay(
        &self,
        private_key: &str,
        network: &str,
        method: Method,
        url: &str,
        headers: Vec<(String, String)>,
        body: Option<String>,
    ) -> Result<(reqwest::Response, Option<String>)> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .build()
            .map_err(|e| GradienceError::Http(format!("client build failed: {}", e)))?;

        // 1) Initial request
        let mut req = client.request(method.clone(), url);
        for (k, v) in &headers {
            req = req.header(k, v);
        }
        if let Some(ref b) = body {
            req = req.body(b.clone());
        }
        let initial = req.send().await.map_err(|e| {
            GradienceError::Http(format!("initial request failed: {}", e))
        })?;

        if initial.status() != reqwest::StatusCode::PAYMENT_REQUIRED {
            return Ok((initial, None));
        }

        // 2) Build bridge input from 402 response
        let status = initial.status().as_u16();
        let resp_headers: HashMap<String, String> = initial
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                Some((k.to_string(), v.to_str().ok()?.to_string()))
            })
            .collect();
        let resp_body: serde_json::Value = initial
            .json()
            .await
            .unwrap_or(serde_json::Value::Null);

        let input = json!({
            "privateKey": private_key,
            "network": network,
            "response": {
                "status": status,
                "headers": resp_headers,
                "body": resp_body,
            }
        });

        // 3) Call Node.js bridge
        let output = tokio::process::Command::new("node")
            .arg("index.mjs")
            .arg(input.to_string())
            .current_dir(&self.bridge_dir)
            .output()
            .await
            .map_err(|e| {
                GradienceError::Validation(format!(
                    "failed to spawn x402 bridge: {} (is Node.js installed?)",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GradienceError::Validation(format!(
                "x402 bridge failed: {}",
                stderr
            )));
        }

        let result: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| {
                GradienceError::Validation(format!(
                    "x402 bridge output parse error: {}",
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
                "x402 payment creation failed: {}",
                err
            )));
        }

        let payment_headers = result["headers"].as_object().ok_or_else(|| {
            GradienceError::Validation("missing headers from x402 bridge".into())
        })?;

        // 4) Retry request with payment signature headers
        let mut retry = client.request(method, url);
        for (k, v) in &headers {
            retry = retry.header(k, v);
        }
        for (k, v) in payment_headers {
            if let Some(val) = v.as_str() {
                retry = retry.header(k, val);
            }
        }
        if let Some(b) = body {
            retry = retry.body(b);
        }
        let paid_response = retry.send().await.map_err(|e| {
            GradienceError::Http(format!("retried request failed: {}", e))
        })?;

        // 5) Extract settlement tx hash from Payment-Response header
        let tx_hash = paid_response
            .headers()
            .get("payment-response")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| {
                let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s).ok()?;
                let json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
                json.get("transaction")
                    .and_then(|t| t.as_str())
                    .map(|t| t.to_string())
            });

        Ok((paid_response, tx_hash))
    }
}

/// Convenience helper: derive a Stellar private key (S...) from a 32-byte seed.
pub fn stellar_secret_from_seed(seed: &[u8; 32]) -> String {
    crate::ows::signing::stellar_secret_from_seed(seed)
}

/// Convenience helper: derive a Stellar address (G...) from a 32-byte seed.
pub fn stellar_address_from_seed(seed: &[u8; 32]) -> Result<String> {
    crate::ows::signing::stellar_address_from_secret_key(seed)
}
