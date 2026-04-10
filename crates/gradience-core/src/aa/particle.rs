use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Particle Network Account Abstraction Enhanced API client.
///
/// Docs: https://developers.particle.network/aa/rpc
#[derive(Debug, Clone)]
pub struct ParticleClient {
    client: reqwest::Client,
    rpc_url: String,
    auth_header: String,
}

impl ParticleClient {
    pub fn new(project_id: &str, client_key: &str) -> Self {
        let creds = base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", project_id, client_key));
        Self {
            client: reqwest::Client::new(),
            rpc_url: "https://rpc.particle.network/evm-chain".into(),
            auth_header: format!("Basic {}", creds),
        }
    }

    /// Retrieve the smart account address(es) for a given owner.
    pub async fn get_smart_account(
        &self,
        chain_id: u64,
        configs: Vec<AccountConfig>,
    ) -> anyhow::Result<Vec<SmartAccountInfo>> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "chainId": chain_id,
            "method": "particle_aa_getSmartAccount",
            "params": [configs],
        });

        self.post_and_extract_result(body).await
    }

    /// Create session key transactions / UserOperations.
    pub async fn create_sessions(
        &self,
        chain_id: u64,
        account: AccountConfig,
        sessions: Vec<SessionDef>,
    ) -> anyhow::Result<CreateSessionsResult> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "chainId": chain_id,
            "method": "particle_aa_createSessions",
            "params": [account, sessions],
        });

        self.post_and_extract_result(body).await
    }

    /// Send a signed UserOperation through Particle’s enhanced API.
    ///
    /// `sessions_opt` may contain `sessions` and `targetSession` definitions.
    pub async fn send_user_op(
        &self,
        chain_id: u64,
        account: AccountConfig,
        user_op: serde_json::Value,
        sessions_opt: Option<serde_json::Value>,
    ) -> anyhow::Result<String> {
        let params = if let Some(s) = sessions_opt {
            vec![json!(account), user_op, s]
        } else {
            vec![json!(account), user_op]
        };

        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "chainId": chain_id,
            "method": "particle_aa_sendUserOp",
            "params": params,
        });

        let json: serde_json::Value = self.post_raw(body).await?;
        if let Some(err) = json.get("error") {
            anyhow::bail!("Particle RPC error: {}", err);
        }

        let result = json
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing or non-string result from particle_aa_sendUserOp"))?
            .to_string();

        Ok(result)
    }

    async fn post_raw(
        &self,
        body: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let resp = self
            .client
            .post(&self.rpc_url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Particle HTTP error ({}): {}", status, text);
        }

        Ok(resp.json().await?)
    }

    async fn post_and_extract_result<T: serde::de::DeserializeOwned>(
        &self,
        body: serde_json::Value,
    ) -> anyhow::Result<T> {
        let json = self.post_raw(body).await?;
        if let Some(err) = json.get("error") {
            anyhow::bail!("Particle RPC error: {}", err);
        }
        let result = json
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Missing result from Particle API"))?;
        Ok(serde_json::from_value(result)?)
    }
}

/// Account config used by Particle AA APIs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountConfig {
    pub name: String,
    pub version: String,
    pub owner_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub biconomy_api_key: Option<String>,
}

/// Response item from `particle_aa_getSmartAccount`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartAccountInfo {
    pub smart_account_address: String,
}

/// Session key definition for `particle_aa_createSessions`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDef {
    pub valid_until: u64,
    pub valid_after: u64,
    pub session_validation_module: String,
    /// Flexible ABI-encoded session key data.
    /// Example: `[ ["address","address","address","uint256"], ["0x...","0x...","0x...",1] ]`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_key_data_in_abi: Option<serde_json::Value>,
}

/// Response from `particle_aa_createSessions`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionsResult {
    #[serde(default)]
    pub verifying_paymaster_gasless: serde_json::Value,
    #[serde(default)]
    pub verifying_paymaster_native: serde_json::Value,
    #[serde(default)]
    pub token_paymaster: serde_json::Value,
    #[serde(default)]
    pub sessions: Vec<serde_json::Value>,
    #[serde(default)]
    pub transactions: Vec<serde_json::Value>,
}
