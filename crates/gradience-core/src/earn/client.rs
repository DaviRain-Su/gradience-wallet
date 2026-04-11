use serde::Deserialize;

/// Minimal LI.FI Earn API client.
///
/// Docs: https://docs.li.fi/earn/quickstart
#[derive(Debug, Clone)]
pub struct EarnClient {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    composer_url: String,
}

/// High-level vault description (best-effort typed subset).
/// Missing fields gracefully ignored via #[serde(default)].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vault {
    pub chain_id: u64,
    pub protocol_name: String,
    pub name: Option<String>,
    pub asset: Asset,
    pub apy: Option<String>,
    pub contract_address: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

/// User position in a vault.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub chain_id: u64,
    pub protocol_name: String,
    pub asset: Asset,
    pub balance_usd: Option<String>,
    pub balance_native: Option<String>,
}

impl EarnClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            base_url: "https://earn.li.fi".into(),
            composer_url: "https://li.quest".into(),
        }
    }

    /// macOS + reqwest 0.11 native-tls 在 Cloudflare HTTP/2 下偶发
    /// "bad protocol version"，curl 工作正常，故作 fallback。
    fn curl_get_json(&self, url: &str) -> anyhow::Result<serde_json::Value> {
        let mut cmd = std::process::Command::new("curl");
        cmd.arg("-s").arg("-L").arg(url);
        if !self.api_key.is_empty() {
            cmd.arg("-H")
                .arg(format!("x-lifi-api-key: {}", self.api_key));
        }
        let output = cmd.output()?;
        if !output.status.success() {
            anyhow::bail!("curl failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(serde_json::from_slice(&output.stdout)?)
    }

    /// Discover yield vaults on a given chain.
    /// Returns raw JSON first; use `parse_vaults` to extract typed data.
    pub async fn discover_vaults_raw(
        &self,
        chain_id: u64,
        limit: Option<usize>,
    ) -> anyhow::Result<serde_json::Value> {
        let mut url = format!("{}/v1/earn/vaults?chainId={}", self.base_url, chain_id);
        if let Some(l) = limit {
            url.push_str(&format!("&limit={}", l));
        }
        url.push_str("&sortBy=apy");
        // Fallback to curl on macOS native-tls issues
        tokio::task::spawn_blocking({
            let url = url.clone();
            let api_key = self.api_key.clone();
            move || {
                let mut cmd = std::process::Command::new("curl");
                cmd.arg("-s").arg("-L").arg(&url);
                if !api_key.is_empty() {
                    cmd.arg("-H").arg(format!("x-lifi-api-key: {}", api_key));
                }
                let output = cmd.output()?;
                if !output.status.success() {
                    anyhow::bail!("curl failed: {}", String::from_utf8_lossy(&output.stderr));
                }
                Ok(serde_json::from_slice(&output.stdout)?)
            }
        }).await?
    }

    /// Typed wrapper over `discover_vaults_raw`.
    pub async fn discover_vaults(
        &self,
        chain_id: u64,
        limit: Option<usize>,
    ) -> anyhow::Result<Vec<Vault>> {
        let raw = self.discover_vaults_raw(chain_id, limit).await?;
        let vaults = raw
            .get("vaults")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let parsed: Vec<Vault> = vaults
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        Ok(parsed)
    }

    /// Fetch user positions across all earn protocols.
    pub async fn get_positions_raw(
        &self,
        wallet_address: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let url = format!(
            "{}/v1/earn/portfolio/{}/positions",
            self.base_url, wallet_address
        );
        tokio::task::spawn_blocking({
            let url = url.clone();
            let api_key = self.api_key.clone();
            move || {
                let mut cmd = std::process::Command::new("curl");
                cmd.arg("-s").arg("-L").arg(&url);
                if !api_key.is_empty() {
                    cmd.arg("-H").arg(format!("x-lifi-api-key: {}", api_key));
                }
                let output = cmd.output()?;
                if !output.status.success() {
                    anyhow::bail!("curl failed: {}", String::from_utf8_lossy(&output.stderr));
                }
                Ok(serde_json::from_slice(&output.stdout)?)
            }
        }).await?
    }

    /// Typed wrapper over `get_positions_raw`.
    pub async fn get_positions(&self, wallet_address: &str) -> anyhow::Result<Vec<Position>> {
        let raw = self.get_positions_raw(wallet_address).await?;
        let positions = raw
            .get("positions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let parsed: Vec<Position> = positions
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        Ok(parsed)
    }

    /// Get a Composer quote for depositing into a vault.
    /// `to_token` should be the vault contract address.
    pub async fn quote_deposit(
        &self,
        from_chain: u64,
        to_chain: u64,
        from_token: &str,
        to_token: &str,
        from_address: &str,
        to_address: &str,
        from_amount: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let url = format!(
            "{}/v1/quote?fromChain={}&toChain={}&fromToken={}&toToken={}&fromAddress={}&toAddress={}&fromAmount={}",
            self.composer_url,
            from_chain,
            to_chain,
            from_token,
            to_token,
            from_address,
            to_address,
            from_amount
        );
        tokio::task::spawn_blocking({
            let url = url.clone();
            let api_key = self.api_key.clone();
            move || {
                let mut cmd = std::process::Command::new("curl");
                cmd.arg("-s").arg("-L").arg(&url);
                if !api_key.is_empty() {
                    cmd.arg("-H").arg(format!("x-lifi-api-key: {}", api_key));
                }
                let output = cmd.output()?;
                if !output.status.success() {
                    anyhow::bail!("curl failed: {}", String::from_utf8_lossy(&output.stderr));
                }
                Ok(serde_json::from_slice(&output.stdout)?)
            }
        }).await?
    }
}
