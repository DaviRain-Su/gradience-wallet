use crate::error::{GradienceError, Result};
use serde::Deserialize;

#[derive(Clone)]
pub struct TonRpcClient {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct TonResponse<T> {
    ok: bool,
    result: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AddressInfo {
    balance: String,
    #[serde(rename = "account_state")]
    _account_state: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct WalletInfo {
    seqno: u32,
}

impl TonRpcClient {
    pub fn new(mainnet: bool) -> Self {
        let base_url = if mainnet {
            "https://toncenter.com/api/v2"
        } else {
            "https://testnet.toncenter.com/api/v2"
        }
        .to_string();
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }

    pub fn new_with_url(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn get_balance(&self, address: &str) -> Result<u128> {
        let mut url = reqwest::Url::parse(&format!("{}/getAddressInformation", self.base_url))
            .map_err(|e| GradienceError::Blockchain(format!("ton rpc invalid url: {}", e)))?;
        url.query_pairs_mut().append_pair("address", address);
        let resp = self.client.get(url).send().await.map_err(|e| {
            GradienceError::Blockchain(format!("ton rpc get_balance failed: {}", e))
        })?;
        let text = resp.text().await.map_err(|e| {
            GradienceError::Blockchain(format!("ton rpc get_balance read text failed: {}", e))
        })?;
        let body: TonResponse<AddressInfo> = serde_json::from_str(&text).map_err(|e| {
            GradienceError::Blockchain(format!("ton rpc get_balance decode failed: {} | raw: {}", e, text.chars().take(200).collect::<String>()))
        })?;
        if !body.ok {
            return Err(GradienceError::Blockchain(format!(
                "ton rpc get_balance error: {:?}",
                body.error
            )));
        }
        let info = body.result.ok_or_else(|| {
            GradienceError::Blockchain("ton rpc get_balance empty result".into())
        })?;
        let balance = info.balance.parse::<u128>().map_err(|e| {
            GradienceError::Blockchain(format!("ton rpc invalid balance: {}", e))
        })?;
        Ok(balance)
    }

    pub async fn get_seqno(&self, address: &str) -> Result<u32> {
        let mut url = reqwest::Url::parse(&format!("{}/getWalletInformation", self.base_url))
            .map_err(|e| GradienceError::Blockchain(format!("ton rpc invalid url: {}", e)))?;
        url.query_pairs_mut().append_pair("address", address);
        let resp = self.client.get(url).send().await.map_err(|e| {
            GradienceError::Blockchain(format!("ton rpc get_seqno failed: {}", e))
        })?;
        let text = resp.text().await.map_err(|e| {
            GradienceError::Blockchain(format!("ton rpc get_seqno read text failed: {}", e))
        })?;
        let body: TonResponse<WalletInfo> = serde_json::from_str(&text).map_err(|e| {
            GradienceError::Blockchain(format!("ton rpc get_seqno decode failed: {} | raw: {}", e, text.chars().take(200).collect::<String>()))
        })?;
        if !body.ok {
            return Err(GradienceError::Blockchain(format!(
                "ton rpc get_seqno error: {:?}",
                body.error
            )));
        }
        let info = body.result.ok_or_else(|| {
            GradienceError::Blockchain("ton rpc get_seqno empty result".into())
        })?;
        Ok(info.seqno)
    }

    pub async fn send_boc(&self, boc_bytes: &[u8]) -> Result<String> {
        let url = format!("{}/sendBoc", self.base_url);
        let boc_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, boc_bytes);
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({ "boc": boc_b64 }))
            .send()
            .await
            .map_err(|e| GradienceError::Blockchain(format!("ton rpc send_boc failed: {}", e)))?;
        let body = resp.json::<TonResponse<serde_json::Value>>().await.map_err(|e| {
            GradienceError::Blockchain(format!("ton rpc send_boc decode failed: {}", e))
        })?;
        if !body.ok {
            return Err(GradienceError::Blockchain(format!(
                "ton rpc send_boc error: {:?}",
                body.error
            )));
        }
        // toncenter returns { "@type": "ok", "@extra": "..." }
        Ok("sent".into())
    }
}
