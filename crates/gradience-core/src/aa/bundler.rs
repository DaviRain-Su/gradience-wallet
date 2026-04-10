use alloy::primitives::B256;
use alloy_rpc_types_eth::erc4337::{SendUserOperationResponse, UserOperation, UserOperationReceipt};
use serde_json::json;

/// Minimal Bundler client for EntryPoint v0.6.
#[derive(Debug, Clone)]
pub struct BundlerClient {
    url: String,
    entry_point: String,
    auth_header: Option<String>,
    client: reqwest::Client,
}

impl BundlerClient {
    pub fn new(
        url: impl Into<String>,
        entry_point: impl Into<String>,
        auth_header: Option<String>,
    ) -> Self {
        Self {
            url: url.into(),
            entry_point: entry_point.into(),
            auth_header,
            client: reqwest::Client::new(),
        }
    }

    /// Submit a user operation to the bundler.
    pub async fn send_user_operation(
        &self,
        op: &UserOperation,
    ) -> anyhow::Result<SendUserOperationResponse> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_sendUserOperation",
            "params": [op, self.entry_point],
        });

        let mut req = self.client.post(&self.url).json(&body);
        if let Some(auth) = &self.auth_header {
            req = req.header("Authorization", auth.clone());
        }
        let resp = req.send().await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Bundler HTTP error: {}", text);
        }

        let json: serde_json::Value = resp.json().await?;
        if let Some(err) = json.get("error") {
            anyhow::bail!("Bundler RPC error: {}", err);
        }

        let result = json
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Missing result from bundler"))?;

        let parsed: SendUserOperationResponse = serde_json::from_value(result)?;
        Ok(parsed)
    }

    /// Poll for the receipt of a submitted userOp.
    pub async fn get_user_operation_receipt(
        &self,
        user_op_hash: &B256,
    ) -> anyhow::Result<Option<UserOperationReceipt>> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getUserOperationReceipt",
            "params": [format!("0x{}", hex::encode(user_op_hash.as_slice()))],
        });

        let mut req = self.client.post(&self.url).json(&body);
        if let Some(auth) = &self.auth_header {
            req = req.header("Authorization", auth.clone());
        }
        let resp = req.send().await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Bundler HTTP error: {}", text);
        }

        let json: serde_json::Value = resp.json().await?;
        if let Some(err) = json.get("error") {
            anyhow::bail!("Bundler RPC error: {}", err);
        }

        let result = json.get("result").cloned();
        match result {
            Some(serde_json::Value::Null) => Ok(None),
            Some(v) => {
                let parsed: UserOperationReceipt = serde_json::from_value(v)?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }
}
