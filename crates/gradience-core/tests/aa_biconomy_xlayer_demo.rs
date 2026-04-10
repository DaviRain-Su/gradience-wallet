use alloy::primitives::{Address, Bytes, U256, address, b256};
use alloy::signers::local::PrivateKeySigner;
use gradience_core::aa::biconomy::BiconomyAccount;
use gradience_core::aa::user_op::UserOpBuilder;

#[tokio::test]
async fn demo_biconomy_xlayer_counterfactual_and_sign() {
    // X Layer Mainnet config
    let rpc_url = "https://rpc.xlayer.tech";
    let factory: Address = address!("000000a56Aaca3e9a4C479ea6b6CD0DbcB6634F5");
    let ecdsa_module: Address = address!("0000001c5b32F37F5beA87BDD5374eB2aC54eA8e");
    let entry_point: Address = address!("5FF137D4b0FDCD49DcA30c7CF57E578a026d2789");
    let chain_id = 196u64;

    // Deterministic owner for demo
    let signer =
        PrivateKeySigner::from_bytes(
            &b256!(
                "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            )
        )
        .unwrap();
    let owner = signer.address();
    let index = U256::ZERO;

    // 1. Query counterfactual address from on-chain factory
    let account = BiconomyAccount::get_counterfactual_address(
        rpc_url,
        factory,
        ecdsa_module,
        owner,
        index,
    )
    .await
    .unwrap();

    println!("Owner: {}", owner);
    println!("Biconomy V2 SmartAccount on X Layer: {}", account);

    // 2. Build initCode (used when account is not yet deployed)
    let init_code =
        BiconomyAccount::build_init_code(factory, ecdsa_module, owner, index);
    println!("InitCode length: {} bytes", init_code.len());

    // 3. Build execute call data (send 0 value to self with empty data as a no-op demo)
    let call_data = BiconomyAccount::build_execute_call_data(
        account, // dest
        U256::ZERO,
        Bytes::new(),
    );

    // 4. Build UserOp
    let mut op = UserOpBuilder::new_v06(
        account,
        U256::ZERO, // nonce (simplified; real nonce should be fetched from entryPoint.getNonce)
        init_code,
        call_data,
        U256::from(1000),
        U256::from(100),
    );

    // 5. Sign with Biconomy V2 signature format
    BiconomyAccount::sign_user_op(
        &mut op,
        &signer,
        entry_point,
        chain_id,
        ecdsa_module,
    )
    .await
    .unwrap();

    println!("UserOp Hash: {}", UserOpBuilder::hash_v06(&op, entry_point, chain_id));
    println!("Signature length: {} bytes", op.signature.len());
    println!("Signature prefix (hex): 0x{}", hex::encode(&op.signature[..32.min(op.signature.len())]));
}
