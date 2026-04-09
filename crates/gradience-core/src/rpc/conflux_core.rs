use crate::error::{GradienceError, Result};
use serde::Deserialize;
use std::process::Command;

fn workspace_dir() -> std::path::PathBuf {
    let manifest = std::env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest)
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn derive_script() -> std::path::PathBuf {
    workspace_dir().join("bridge/conflux-core/src/derive.js")
}

fn sign_script() -> std::path::PathBuf {
    workspace_dir().join("bridge/conflux-core/src/sign.js")
}

#[derive(Debug, Deserialize)]
struct DeriveOutput {
    #[serde(default)]
    success: bool,
    address: Option<String>,
    #[serde(rename = "hexAddress")]
    _hex_address: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SignOutput {
    #[serde(default)]
    success: bool,
    #[serde(rename = "txHash")]
    tx_hash: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

pub struct ConfluxCoreRpcClient {
    rpc_url: String,
}

impl ConfluxCoreRpcClient {
    pub fn new_with_url(rpc_url: &str) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
        }
    }

    pub async fn get_balance(&self, address: &str) -> Result<u128> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "cfx_getBalance",
            "params": [address, "latest_state"],
            "id": 1,
        });
        let resp = reqwest::Client::new()
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| GradienceError::Http(format!("conflux core rpc post failed: {}", e)))?;
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e: reqwest::Error| GradienceError::Http(e.to_string()))?;
        if let Some(err) = json.get("error") {
            return Err(GradienceError::Http(format!(
                "conflux core rpc error: {}",
                err
            )));
        }
        let result = json["result"].as_str().unwrap_or("0x0");
        let drip = u128::from_str_radix(result.trim_start_matches("0x"), 16)
            .map_err(|_| GradienceError::Validation(format!("invalid balance hex: {}", result)))?;
        Ok(drip)
    }

    pub fn derive_address(seed: &[u8], network_id: u32) -> Result<String> {
        let seed_hex = format!("0x{}", hex::encode(seed));
        let output = Command::new("node")
            .arg(derive_script())
            .arg(&seed_hex)
            .arg(network_id.to_string())
            .output()
            .map_err(|e| {
                GradienceError::Blockchain(format!(
                    "failed to run conflux-core derive bridge: {}",
                    e
                ))
            })?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: DeriveOutput = serde_json::from_str(&stdout).map_err(|e| {
            GradienceError::Blockchain(format!(
                "conflux-core derive bridge invalid json: {} (stdout: {})",
                e, stdout
            ))
        })?;
        if !parsed.success {
            return Err(GradienceError::Blockchain(format!(
                "conflux-core derive bridge failed: {}",
                parsed.error.unwrap_or_default()
            )));
        }
        parsed.address.ok_or_else(|| {
            GradienceError::Blockchain("conflux-core derive bridge missing address".into())
        })
    }

    pub fn sign_and_send(
        &self,
        private_key: &str,
        to: &str,
        value_hex: &str,
        network_id: u32,
    ) -> Result<String> {
        let output = Command::new("node")
            .arg(sign_script())
            .arg("--rpc")
            .arg(&self.rpc_url)
            .arg("--privateKey")
            .arg(private_key)
            .arg("--to")
            .arg(to)
            .arg("--value")
            .arg(value_hex)
            .arg("--networkId")
            .arg(network_id.to_string())
            .output()
            .map_err(|e| {
                GradienceError::Blockchain(format!("failed to run conflux-core sign bridge: {}", e))
            })?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let parsed: SignOutput = serde_json::from_str(&stdout).map_err(|e| {
            GradienceError::Blockchain(format!(
                "conflux-core sign bridge invalid json: {} (stdout: {} stderr: {})",
                e, stdout, stderr
            ))
        })?;
        if !parsed.success {
            return Err(GradienceError::Blockchain(format!(
                "conflux-core sign bridge failed: {}",
                parsed.error.unwrap_or_default()
            )));
        }
        parsed.tx_hash.ok_or_else(|| {
            GradienceError::Blockchain("conflux-core sign bridge missing tx_hash".into())
        })
    }
}

// sync helper used by un-async functions like derive_account
pub fn cfx_address_from_seed(seed: &[u8], network_id: u32) -> Result<String> {
    ConfluxCoreRpcClient::derive_address(seed, network_id)
}
