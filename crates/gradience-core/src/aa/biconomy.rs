use alloy::primitives::{Address, Bytes, U256};
use alloy::signers::Signer;
use alloy::sol_types::{sol, SolCall, SolValue};

sol! {
    contract SmartAccountFactory {
        function getAddressForCounterFactualAccount(address moduleSetupContract, bytes moduleSetupData, uint256 index) external view returns (address);
        function deployCounterFactualAccount(address moduleSetupContract, bytes moduleSetupData, uint256 index) external returns (address);
        function basicImplementation() external view returns (address);
    }
}

sol! {
    contract EcdsaOwnershipRegistryModule {
        function initForSmartAccount(address eoaOwner) external returns (address);
    }
}

sol! {
    contract SmartAccount {
        function execute(address dest, uint256 value, bytes func) external;
        function executeBatch(address[] dest, uint256[] value, bytes[] func) external;
    }
}

/// Biconomy V2 Account Abstraction helpers.
pub struct BiconomyAccount;

impl BiconomyAccount {
    /// Query the counterfactual Smart Account address via on-chain factory.
    pub async fn get_counterfactual_address(
        rpc_url: &str,
        factory: Address,
        ecdsa_module: Address,
        owner: Address,
        index: U256,
    ) -> anyhow::Result<Address> {
        let module_setup_data =
            EcdsaOwnershipRegistryModule::initForSmartAccountCall { eoaOwner: owner }.abi_encode();
        let call_data = SmartAccountFactory::getAddressForCounterFactualAccountCall {
            moduleSetupContract: ecdsa_module,
            moduleSetupData: Bytes::from(module_setup_data),
            index,
        }
        .abi_encode();

        let result_hex = eth_call(rpc_url, factory, Bytes::from(call_data)).await?;
        if result_hex.len() != 32 {
            anyhow::bail!("unexpected eth_call return length: {}", result_hex.len());
        }
        Ok(Address::from_slice(&result_hex[12..]))
    }

    /// Build the `initCode` for deploying a Biconomy V2 Smart Account via ERC-4337.
    pub fn build_init_code(
        factory: Address,
        ecdsa_module: Address,
        owner: Address,
        index: U256,
    ) -> Bytes {
        let module_setup_data =
            EcdsaOwnershipRegistryModule::initForSmartAccountCall { eoaOwner: owner }.abi_encode();

        let factory_calldata = SmartAccountFactory::deployCounterFactualAccountCall {
            moduleSetupContract: ecdsa_module,
            moduleSetupData: Bytes::from(module_setup_data),
            index,
        }
        .abi_encode();

        let mut init_code = Vec::with_capacity(20 + factory_calldata.len());
        init_code.extend_from_slice(factory.as_slice());
        init_code.extend_from_slice(&factory_calldata);
        Bytes::from(init_code)
    }

    /// Encode a Biconomy V2 UserOp signature.
    /// Format: `abi.encode(raw_ecdsa_signature, validation_module_address)`
    pub fn encode_signature(sig: &[u8], validation_module: Address) -> Bytes {
        let sig_bytes = Bytes::from(sig.to_vec());
        (sig_bytes, validation_module).abi_encode_sequence().into()
    }

    /// Build `execute(dest, value, func)` call data.
    pub fn build_execute_call_data(dest: Address, value: U256, func: Bytes) -> Bytes {
        SmartAccount::executeCall { dest, value, func }.abi_encode().into()
    }

    /// Build a signed UserOp for Biconomy V2 (no session key).
    pub async fn sign_user_op(
        op: &mut alloy_rpc_types_eth::erc4337::UserOperation,
        signer: &impl Signer,
        entry_point: Address,
        chain_id: u64,
        validation_module: Address,
    ) -> anyhow::Result<()> {
        let hash =
            crate::aa::user_op::UserOpBuilder::hash_v06(op, entry_point, chain_id);
        let sig = signer.sign_hash(&hash).await?;
        let raw = sig.as_bytes();
        op.signature = Self::encode_signature(&raw[..], validation_module);
        Ok(())
    }
}

async fn eth_call(rpc_url: &str, to: Address, data: Bytes) -> anyhow::Result<Bytes> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_call",
        "params": [
            {
                "to": format!("0x{}", hex::encode(to.as_slice())),
                "data": format!("0x{}", hex::encode(&data))
            },
            "latest"
        ]
    });

    let resp = reqwest::Client::new()
        .post(rpc_url)
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("RPC HTTP error ({}): {}", status, text);
    }

    let json: serde_json::Value = resp.json().await?;
    if let Some(err) = json.get("error") {
        anyhow::bail!("RPC error: {}", err);
    }

    let result = json
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing result from eth_call"))?;

    let bytes = hex::decode(result.trim_start_matches("0x"))?;
    Ok(Bytes::from(bytes))
}
