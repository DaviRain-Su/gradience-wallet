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

#[derive(Debug, Deserialize)]
pub struct RunGetMethodResult {
    exit_code: i64,
    stack: Vec<Vec<serde_json::Value>>,
    #[serde(rename = "gas_used")]
    _gas_used: Option<u64>,
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

    /// Call a get-method on a TON contract.
    pub async fn run_get_method(
        &self,
        address: &str,
        method: &str,
        stack: Vec<serde_json::Value>,
    ) -> Result<RunGetMethodResult> {
        let url = format!("{}/runGetMethod", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "address": address,
                "method": method,
                "stack": stack,
            }))
            .send()
            .await
            .map_err(|e| GradienceError::Blockchain(format!("ton rpc runGetMethod failed: {}", e)))?;
        let text = resp.text().await.map_err(|e| {
            GradienceError::Blockchain(format!("ton rpc runGetMethod read text failed: {}", e))
        })?;
        let body: TonResponse<RunGetMethodResult> = serde_json::from_str(&text).map_err(|e| {
            GradienceError::Blockchain(format!(
                "ton rpc runGetMethod decode failed: {} | raw: {}",
                e,
                text.chars().take(200).collect::<String>()
            ))
        })?;
        if !body.ok {
            return Err(GradienceError::Blockchain(format!(
                "ton rpc runGetMethod error: {:?}",
                body.error
            )));
        }
        let result = body.result.ok_or_else(|| {
            GradienceError::Blockchain("ton rpc runGetMethod empty result".into())
        })?;
        if result.exit_code != 0 {
            return Err(GradienceError::Blockchain(format!(
                "ton rpc runGetMethod exit_code: {}",
                result.exit_code
            )));
        }
        Ok(result)
    }

    /// Get Jetton wallet address for a given owner and Jetton master.
    pub async fn get_jetton_wallet_address(
        &self,
        jetton_master: &str,
        owner_address: &str,
    ) -> Result<String> {
        // Encode owner address as tvm.Slice (base64 BoC)
        let owner: tlb_ton::MsgAddress = owner_address
            .parse()
            .map_err(|e: <tlb_ton::MsgAddress as std::str::FromStr>::Err| {
                GradienceError::Blockchain(format!("invalid owner ton address: {}", e))
            })?;
        let mut builder = tlb_ton::Cell::builder();
        use tlb_ton::bits::ser::BitWriterExt;
        builder
            .pack(owner, ())
            .map_err(|e| GradienceError::Blockchain(format!("ton cell pack failed: {}", e)))?;
        let cell = builder.into_cell();
        let boc = tlb_ton::BoC::from_root(cell);
        let boc_bytes = boc
            .serialize(tlb_ton::BagOfCellsArgs {
                has_idx: false,
                has_crc32c: true,
            })
            .map_err(|e| GradienceError::Blockchain(format!("ton boc serialize failed: {}", e)))?;
        let owner_boc_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &boc_bytes,
        );

        let result = self
            .run_get_method(
                jetton_master,
                "get_wallet_address",
                vec![serde_json::json!([
                    "tvm.Slice",
                    owner_boc_b64
                ])],
            )
            .await?;

        let slice_b64 = result
            .stack
            .first()
            .and_then(|item| item.get(1))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                GradienceError::Blockchain("runGetMethod stack missing slice value".into())
            })?;

        let boc_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            slice_b64,
        )
        .map_err(|e| GradienceError::Blockchain(format!("invalid base64 slice: {}", e)))?;
        let boc = tlb_ton::BoC::deserialize(&boc_bytes)
            .map_err(|e| GradienceError::Blockchain(format!("ton boc deserialize failed: {}", e)))?;
        let cell = boc.into_single_root().ok_or_else(|| {
            GradienceError::Blockchain("ton boc missing root cell".into())
        })?;
        let addr: tlb_ton::MsgAddress = tlb_ton::bits::de::unpack(&cell.data, ())
            .map_err(|e| GradienceError::Blockchain(format!("ton address unpack failed: {}", e)))?;
        Ok(addr.to_base64_url())
    }

    /// Get Jetton wallet balance.
    pub async fn get_jetton_balance(&self, jetton_wallet: &str) -> Result<u128> {
        let result = self
            .run_get_method(jetton_wallet, "get_wallet_data", vec![])
            .await?;
        // get_wallet_data returns: [balance, owner_address, jetton_master_address, wallet_code]
        let balance_str = result
            .stack
            .first()
            .and_then(|item| item.first())
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                GradienceError::Blockchain("runGetMethod stack missing balance num".into())
            })?;
        let balance = balance_str
            .parse::<u128>()
            .map_err(|e| GradienceError::Blockchain(format!("invalid jetton balance: {}", e)))?;
        Ok(balance)
    }
}
