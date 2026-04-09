use crate::error::{GradienceError, Result};
use serde_json::json;

#[derive(Debug)]
pub struct EvmRpcClient {
    _chain_id: String,
    rpc_url: String,
    client: reqwest::Client,
}

impl EvmRpcClient {
    pub fn new(chain_id: &str, rpc_url: &str) -> Result<Self> {
        let url = reqwest::Url::parse(rpc_url).map_err(|e| GradienceError::Http(e.to_string()))?;
        Ok(Self {
            _chain_id: chain_id.into(),
            rpc_url: url.to_string(),
            client: reqwest::Client::new(),
        })
    }

    pub async fn get_balance(&self, address: &str) -> Result<String> {
        let resp = self
            .call("eth_getBalance", vec![json!(address), json!("latest")])
            .await?;
        resp.as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| GradienceError::Http("invalid balance response".into()))
    }

    pub async fn send_raw_transaction(&self, raw_tx: &str) -> Result<String> {
        let resp = self
            .call("eth_sendRawTransaction", vec![json!(raw_tx)])
            .await?;
        resp.as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| GradienceError::Http(format!("send_raw_tx failed: {:?}", resp)))
    }

    pub async fn get_gas_price(&self) -> Result<String> {
        let resp = self
            .call("eth_gasPrice", Vec::<serde_json::Value>::new())
            .await?;
        resp.as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| GradienceError::Http("invalid gasPrice response".into()))
    }

    pub async fn get_transaction_count(&self, address: &str) -> Result<u64> {
        let resp = self
            .call(
                "eth_getTransactionCount",
                vec![json!(address), json!("latest")],
            )
            .await?;
        let hex = resp
            .as_str()
            .ok_or_else(|| GradienceError::Http("invalid nonce response".into()))?;
        u64::from_str_radix(hex.trim_start_matches("0x"), 16)
            .map_err(|e| GradienceError::Http(format!("invalid nonce hex: {}", e)))
    }

    pub async fn eth_call(&self, to: &str, data: &str) -> Result<String> {
        let params = json!({
            "to": to,
            "data": data,
        });
        let resp = self.call("eth_call", vec![params, json!("latest")]).await?;
        resp.as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| GradienceError::Http("invalid eth_call response".into()))
    }

    async fn call(
        &self,
        method: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| GradienceError::Http(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(GradienceError::Http(format!("HTTP {} from RPC", status)));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| GradienceError::Http(e.to_string()))?;

        if let Some(err) = body.get("error") {
            return Err(GradienceError::Http(err.to_string()));
        }

        body.get("result")
            .cloned()
            .ok_or_else(|| GradienceError::Http("missing result field".into()))
    }
}
