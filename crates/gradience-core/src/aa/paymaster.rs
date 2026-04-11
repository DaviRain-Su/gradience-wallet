use alloy::primitives::Bytes;
use serde_json::json;

/// Minimal Paymaster client targeting Particle Network's `pm_sponsorUserOperation` endpoint.
///
/// Docs: https://developers.particle.network/aa/paymaster/sponsoruseroperation
#[derive(Debug, Clone)]
pub struct PaymasterClient {
    client: reqwest::Client,
    url: String,
    project_uuid: String,
    project_key: String,
}

/// Result of a successful sponsorship request.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SponsorResult {
    pub paymaster_and_data: String,
}

impl PaymasterClient {
    pub fn new(project_uuid: impl Into<String>, project_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: "https://paymaster.particle.network".into(),
            project_uuid: project_uuid.into(),
            project_key: project_key.into(),
        }
    }

    /// Request paymaster sponsorship for a user operation.
    ///
    /// The `user_op` should have a dummy signature and empty `paymaster_and_data`
    /// when passed in. The returned `paymaster_and_data` must be injected back into
    /// the op before generating the final signature.
    pub async fn sponsor_user_op(
        &self,
        chain_id: u64,
        entry_point: &str,
        user_op: &alloy_rpc_types_eth::erc4337::UserOperation,
    ) -> anyhow::Result<SponsorResult> {
        let user_op_json = json!({
            "sender": user_op.sender.to_string(),
            "nonce": format!("0x{:x}", user_op.nonce),
            "initCode": format!("0x{}", hex::encode(user_op.init_code.as_ref())),
            "callData": format!("0x{}", hex::encode(user_op.call_data.as_ref())),
            "callGasLimit": format!("0x{:x}", user_op.call_gas_limit),
            "verificationGasLimit": format!("0x{:x}", user_op.verification_gas_limit),
            "preVerificationGas": format!("0x{:x}", user_op.pre_verification_gas),
            "maxFeePerGas": format!("0x{:x}", user_op.max_fee_per_gas),
            "maxPriorityFeePerGas": format!("0x{:x}", user_op.max_priority_fee_per_gas),
            "paymasterAndData": format!("0x{}", hex::encode(user_op.paymaster_and_data.as_ref())),
            "signature": format!("0x{}", hex::encode(user_op.signature.as_ref())),
        });

        let body = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "pm_sponsorUserOperation",
            "params": [user_op_json, entry_point],
        });

        let resp = self
            .client
            .post(&self.url)
            .query(&[
                ("chainId", chain_id.to_string()),
                ("projectUuid", self.project_uuid.clone()),
                ("projectKey", self.project_key.clone()),
            ])
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Paymaster HTTP error: {}", text);
        }

        let json: serde_json::Value = resp.json().await?;
        if let Some(err) = json.get("error") {
            anyhow::bail!("Paymaster RPC error: {}", err);
        }

        let result = json
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Missing result from paymaster"))?;

        let parsed: SponsorResult = serde_json::from_value(result)?;
        Ok(parsed)
    }
}

/// Helper to inject `paymaster_and_data` into a UserOperation.
pub fn apply_paymaster(
    op: &mut alloy_rpc_types_eth::erc4337::UserOperation,
    paymaster_data: &str,
) -> Result<(), hex::FromHexError> {
    let bytes = paymaster_data.strip_prefix("0x").unwrap_or(paymaster_data);
    op.paymaster_and_data = Bytes::from(hex::decode(bytes)?);
    Ok(())
}
