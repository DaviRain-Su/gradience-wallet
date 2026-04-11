use alloy::primitives::{Address, U256, address, b256};
use alloy::signers::local::PrivateKeySigner;
use gradience_core::aa::biconomy::BiconomyAccount;
use gradience_core::aa::biconomy_session::BiconomySession;
use gradience_core::aa::particle::{AccountConfig, ParticleClient, SessionDef};
use alloy::sol_types::{sol, SolCall};

sol! {
    interface IERC20 {
        function transfer(address to, uint256 amount) external returns (bool);
    }
}

/// Simplified end-to-end using Particle Enhanced API instead of raw handleOps.
///
/// Requires PARTICLE_PROJECT_ID and PARTICLE_CLIENT_KEY env vars.
#[tokio::test]
async fn particle_biconomy_session_demo() {
    let project_id = std::env::var("PARTICLE_PROJECT_ID").unwrap_or_default();
    let client_key = std::env::var("PARTICLE_CLIENT_KEY").unwrap_or_default();

    if project_id.is_empty() || client_key.is_empty() {
        println!("SKIP: set PARTICLE_PROJECT_ID and PARTICLE_CLIENT_KEY to run.");
        println!("      Get keys at https://dashboard.particle.network");
        return;
    }

    let rpc_url = "https://testrpc.xlayer.tech";
    let factory: Address = address!("92fC61085c34e4F5E03A4DC92CCaFfaaC637C704");
    let ecdsa_module: Address = address!("B4D0Af2926f5147e68bDA3b74d512Cc0A7c2ebAF");
    let _skm_module: Address = address!("0772214738E12e421666A60E62C1aBA9ab766a19");
    let erc20_svm: Address = address!("21475455CB131a8A713e1C629c6f7398f56d504b");
    let chain_id = 1952u64;

    let owner_signer = PrivateKeySigner::from_bytes(
        &b256!("bebff393a40d6aabe1e7fd66bd7299f094255ed574b4abc08f5329b9629ee4c9")
    ).unwrap();
    let owner = owner_signer.address();

    let session_signer = PrivateKeySigner::from_bytes(
        &b256!("deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678")
    ).unwrap();
    let session_key = session_signer.address();

    let index = U256::from(9);
    let account = BiconomyAccount::get_counterfactual_address(
        rpc_url, factory, ecdsa_module, owner, index,
    ).await.unwrap();

    println!("Owner: {}", owner);
    println!("Session Key: {}", session_key);
    println!("SmartAccount: {}", account);

    let token: Address = address!("7220397E4a9AF851C65afe117F91c20222DAFcab");
    let recipient: Address = address!("909E30bdBCb728131E3F8d17150eaE740C904649");
    let max_amount = U256::from(1_000_000);
    let session_key_data = BiconomySession::encode_erc20_session_key_data(
        session_key, token, recipient, max_amount, account, 1000,
    );

    let valid_until = 0u64;
    let valid_after = 0u64;
    let leaf = BiconomySession::leaf_hash(valid_until, valid_after, erc20_svm, &session_key_data);
    let merkle_root = BiconomySession::single_leaf_merkle_root(&leaf);
    println!("Merkle Root: {}", merkle_root);

    let client = ParticleClient::new(&project_id, &client_key);
    let account_config = AccountConfig {
        name: "BICONOMY".into(),
        version: "2.0.0".into(),
        owner_address: owner.to_string(),
        biconomy_api_key: None,
    };

    let mut account_bytes = [0u8; 32];
    account_bytes[12..].copy_from_slice(account.as_slice());
    let max_usage: U256 = (U256::from_be_bytes(account_bytes) << 64) | U256::from(1000u64);

    let session_def = SessionDef {
        valid_until,
        valid_after,
        session_validation_module: erc20_svm.to_string(),
        session_key_data_in_abi: Some(
            serde_json::json!([
                ["address", "address", "address", "uint256", "uint256"],
                [
                    session_key.to_string(),
                    token.to_string(),
                    recipient.to_string(),
                    max_amount.to_string(),
                    max_usage.to_string(),
                ]
            ])
        ),
    };

    let create_result = client
        .create_sessions(chain_id, account_config.clone(), vec![session_def])
        .await
        .expect("create_sessions failed");
    println!("CreateSessions result: {:?}", create_result);

    // Build the session execution userOp via Particle sendUserOp
    let transfer_call = BiconomyAccount::build_execute_call_data(
        token,
        U256::ZERO,
        alloy::primitives::Bytes::from(
            IERC20::transferCall { to: recipient, amount: U256::ZERO }.abi_encode()
        ),
    );

    let user_op = serde_json::json!({
        "sender": account.to_string(),
        "nonce": "0x1",
        "initCode": "0x",
        "callData": format!("0x{}", hex::encode(transfer_call)),
        "callGasLimit": "0x7a120",
        "verificationGasLimit": "0x7a120",
        "preVerificationGas": "0x30d40",
        "maxFeePerGas": "0x77359400",
        "maxPriorityFeePerGas": "0x3b9aca00",
        "paymasterAndData": "0x",
        "signature": "0x",
    });

    let sessions_opt = serde_json::json!({
        "sessions": [{
            "validUntil": valid_until,
            "validAfter": valid_after,
            "sessionValidationModule": erc20_svm.to_string(),
            "sessionKeyData": format!("0x{}", hex::encode(&session_key_data)),
            "merkleRoot": merkle_root.to_string(),
        }],
        "targetSession": {
            "sessionSigner": session_key.to_string(),
            "sessionValidationModule": erc20_svm.to_string(),
            "sessionKeyData": format!("0x{}", hex::encode(&session_key_data)),
            "merkleProof": [leaf.to_string()],
        }
    });

    let tx_hash = client
        .send_user_op(chain_id, account_config, user_op, Some(sessions_opt))
        .await
        .expect("send_user_op failed");
    println!("Particle sendUserOp tx hash: {}", tx_hash);
}
