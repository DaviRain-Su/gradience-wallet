use alloy::primitives::{Address, Bytes, U256, address, b256};
use alloy::signers::Signer;
use alloy::signers::local::PrivateKeySigner;
use alloy::sol_types::{sol, SolCall};
use gradience_core::aa::biconomy::BiconomyAccount;
use gradience_core::aa::biconomy_session::BiconomySession;
use gradience_core::aa::user_op::UserOpBuilder;

sol! {
    struct UserOperation {
        address sender;
        uint256 nonce;
        bytes initCode;
        bytes callData;
        uint256 callGasLimit;
        uint256 verificationGasLimit;
        uint256 preVerificationGas;
        uint256 maxFeePerGas;
        uint256 maxPriorityFeePerGas;
        bytes paymasterAndData;
        bytes signature;
    }

    interface IEntryPoint {
        function handleOps(UserOperation[] calldata ops, address payable beneficiary) external;
    }

    interface ModuleManager {
        function enableModule(address module) external;
    }

    interface SmartAccountBatch {
        function executeBatch(address[] calldata dest, uint256[] calldata value, bytes[] calldata func) external;
    }

    interface IERC20 {
        function transfer(address to, uint256 amount) external returns (bool);
    }
}

fn to_sol_op(op: &alloy_rpc_types_eth::erc4337::UserOperation) -> UserOperation {
    UserOperation {
        sender: op.sender,
        nonce: op.nonce,
        initCode: op.init_code.clone(),
        callData: op.call_data.clone(),
        callGasLimit: op.call_gas_limit,
        verificationGasLimit: op.verification_gas_limit,
        preVerificationGas: op.pre_verification_gas,
        maxFeePerGas: op.max_fee_per_gas,
        maxPriorityFeePerGas: op.max_priority_fee_per_gas,
        paymasterAndData: op.paymaster_and_data.clone(),
        signature: op.signature.clone(),
    }
}

#[tokio::test]
async fn demo_biconomy_async_init() {
    let rpc_url = "https://testrpc.xlayer.tech";
    let entry_point: Address = address!("5FF137D4b0FDCD49DcA30c7CF57E578a026d2789");
    let chain_id = 1952u64;
    let ecdsa_module: Address = address!("B4D0Af2926f5147e68bDA3b74d512Cc0A7c2ebAF");
    let skm_module: Address = address!("0772214738E12e421666A60E62C1aBA9ab766a19");
    let erc20_svm: Address = address!("21475455CB131a8A713e1C629c6f7398f56d504b");
    let factory: Address = address!("92fC61085c34e4F5E03A4DC92CCaFfaaC637C704");

    let owner_signer = PrivateKeySigner::from_bytes(
        &b256!("bebff393a40d6aabe1e7fd66bd7299f094255ed574b4abc08f5329b9629ee4c9")
    ).unwrap();
    let owner = owner_signer.address();

    let session_signer = PrivateKeySigner::from_bytes(
        &b256!("cafebabe1234567890abcdef1234567890abcdef1234567890abcdef12345678")
    ).unwrap();
    let session_key = session_signer.address();

    let index = U256::from(8);
    let account = BiconomyAccount::get_counterfactual_address(
        rpc_url, factory, ecdsa_module, owner, index,
    ).await.unwrap();

    println!("SmartAccount: {}", account);
    println!("Session Key:  {}", session_key);

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

    use alloy::network::EthereumWallet;
    use alloy::providers::Provider;
    let wallet = EthereumWallet::from(owner_signer.clone());
    let provider = alloy::providers::ProviderBuilder::new()
        .wallet(wallet)
        .connect_http(rpc_url.parse().unwrap());

    let balance = provider.get_balance(account).await.unwrap();
    println!("SA balance: {} wei", balance);
    if balance < U256::from(30_000_000_000_000_000u64) {
        println!("Funding SA with 0.04 OKB...");
        let tx = alloy_rpc_types_eth::TransactionRequest::default()
            .to(account)
            .value(U256::from(40_000_000_000_000_000u64));
        let _ = provider.send_transaction(tx).await.unwrap().get_receipt().await.unwrap();
    }

    // ---- Check if SA already deployed (make Phase 1 idempotent) ----
    let already_deployed = match provider.get_code_at(account).await {
        Ok(code) => !code.is_empty(),
        _ => false,
    };

    let phase1_handle = if already_deployed {
        println!("SA already deployed; skipping Phase 1.");
        None
    } else {
        let owner_signer = owner_signer.clone();
        let rpc_url = rpc_url.to_string();
        Some(tokio::spawn(async move {
            let wallet = EthereumWallet::from(owner_signer.clone());
            let provider = alloy::providers::ProviderBuilder::new()
                .wallet(wallet)
                .connect_http(rpc_url.parse::<reqwest::Url>().unwrap());

            let enable_module_data = ModuleManager::enableModuleCall { module: skm_module }.abi_encode();
            let set_root_inner = BiconomySession::build_set_merkle_root_call_data(merkle_root);

            let batch_call = SmartAccountBatch::executeBatchCall {
                dest: vec![account, skm_module],
                value: vec![U256::ZERO, U256::ZERO],
                func: vec![Bytes::from(enable_module_data), set_root_inner],
            }.abi_encode().into();

            let init_code = BiconomyAccount::build_init_code(factory, ecdsa_module, owner, index);

            let mut op_set_root = UserOpBuilder::new_v06(
                account,
                U256::ZERO,
                init_code,
                batch_call,
                U256::from(2_000_000_000u64),
                U256::from(1_000_000_000u64),
            );
            op_set_root.call_gas_limit = U256::from(500_000);
            op_set_root.verification_gas_limit = U256::from(500_000);
            op_set_root.pre_verification_gas = U256::from(200_000);

            BiconomyAccount::sign_user_op(
                &mut op_set_root,
                &owner_signer,
                entry_point,
                chain_id,
                ecdsa_module,
            ).await.unwrap();

            let handle_ops_data = IEntryPoint::handleOpsCall {
                ops: vec![to_sol_op(&op_set_root)],
                beneficiary: owner,
            }.abi_encode();

            let tx = alloy_rpc_types_eth::TransactionRequest::default()
                .to(entry_point)
                .input(handle_ops_data.into())
                .gas_limit(4_000_000);
            let receipt = provider.send_transaction(tx).await.unwrap().get_receipt().await.unwrap();
            println!("Async Phase 1 tx hash: {:?}", receipt.transaction_hash);
            receipt.transaction_hash
        }))
    };

    // ---- Poll / wait for Phase 1 readiness ----
    if let Some(h) = phase1_handle {
        println!("Polling for SA deployment...");
        let mut deployed = false;
        for _ in 0..30 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            match provider.get_code_at(account).await {
                Ok(code) if !code.is_empty() => {
                    println!("SA code detected! {} bytes", code.len());
                    deployed = true;
                    break;
                }
                _ => {
                    println!("SA not yet deployed, retrying...");
                }
            }
        }
        assert!(deployed, "SA did not deploy within timeout");

        let p1_tx = h.await.expect("Phase 1 task panicked");
        println!("Phase 1 completed: {:?}", p1_tx);
    } else {
        println!("Phase 1 skipped; SA already initialized.");
    }

    // Re-create provider so nonce is freshly fetched from chain
    drop(provider);
    let provider = alloy::providers::ProviderBuilder::new()
        .wallet(EthereumWallet::from(owner_signer.clone()))
        .connect_http(rpc_url.parse().unwrap());

    // ---- Phase 2: Session-signed Execute ----
    let transfer_call = IERC20::transferCall { to: recipient, amount: U256::ZERO }
        .abi_encode()
        .into();
    let exec_call = BiconomyAccount::build_execute_call_data(token, U256::ZERO, transfer_call);

    let mut op_exec = UserOpBuilder::new_v06(
        account,
        U256::from(1),
        Bytes::new(),
        exec_call,
        U256::from(2_000_000_000u64),
        U256::from(1_000_000_000u64),
    );
    op_exec.call_gas_limit = U256::from(500_000);
    op_exec.verification_gas_limit = U256::from(500_000);
    op_exec.pre_verification_gas = U256::from(200_000);

    let uo_hash_p2 = UserOpBuilder::hash_v06(&op_exec, entry_point, chain_id);
    let session_sig = session_signer.sign_message(uo_hash_p2.as_slice()).await.unwrap();
    let module_sig = BiconomySession::build_module_signature(
        valid_until,
        valid_after,
        erc20_svm,
        session_key_data,
        vec![],
        &session_sig.as_bytes()[..],
    );
    op_exec.signature = BiconomySession::build_user_op_signature(module_sig, skm_module);

    let handle_ops_data_p2 = IEntryPoint::handleOpsCall {
        ops: vec![to_sol_op(&op_exec)],
        beneficiary: owner,
    }.abi_encode();

    let tx2 = alloy_rpc_types_eth::TransactionRequest::default()
        .to(entry_point)
        .input(handle_ops_data_p2.into())
        .gas_limit(4_000_000);
    let receipt_p2 = provider.send_transaction(tx2).await.unwrap().get_receipt().await.unwrap();
    println!("Phase 2 tx hash: {:?}", receipt_p2.transaction_hash);
}
