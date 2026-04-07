use crate::error::{GradienceError, Result};
use serde_json::json;

#[derive(Debug)]
pub struct EvmRpcClient {
    chain_id: String,
    rpc_url: String,
    client: reqwest::Client,
}

impl EvmRpcClient {
    pub fn new(chain_id: &str, rpc_url: &str) -> Result<Self> {
        let url = reqwest::Url::parse(rpc_url)
            .map_err(|e| GradienceError::Http(e.to_string()))?;
        Ok(Self {
            chain_id: chain_id.into(),
            rpc_url: url.to_string(),
            client: reqwest::Client::new(),
        })
    }

    pub async fn get_balance(&self, address: &str) -> Result<String> {
        let resp = self.call("eth_getBalance", vec![json!(address), json!("latest")]).await?;
        resp.as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| GradienceError::Http("invalid balance response".into()))
    }

    pub async fn send_raw_transaction(&self, raw_tx: &str) -> Result<String> {
        let resp = self.call("eth_sendRawTransaction", vec![json!(raw_tx)]).await?;
        resp.as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| GradienceError::Http(format!("send_raw_tx failed: {:?}", resp)))
    }

    pub async fn get_gas_price(&self) -> Result<String> {
        let resp = self.call("eth_gasPrice", Vec::<serde_json::Value>::new()).await?;
        resp.as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| GradienceError::Http("invalid gasPrice response".into()))
    }

    async fn call(&self,
        method: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let resp = self.client
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
