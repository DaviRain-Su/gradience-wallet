use alloy::primitives::{keccak256, Address, Bytes, U256, b256};
use alloy::signers::local::PrivateKeySigner;
use gradience_core::aa::config::AaConfig;
use gradience_core::aa::factory::simple_account_address;
use gradience_core::aa::user_op::UserOpBuilder;

#[tokio::test]
async fn demo_xlayer_counterfactual_userop() {
    let cfg = AaConfig::for_chain(196).expect("X Layer AA config should exist");
    assert_eq!(cfg.chain_id, 196);

    // Warn if factory is still a placeholder
    if cfg.simple_account_factory == Address::ZERO {
        println!("WARN: simple_account_factory is a placeholder on X Layer; counterfactual address is not deployable yet.");
    }

    // Deterministic owner key for demo
    let signer =
        PrivateKeySigner::from_bytes(
            &b256!(
                "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            )
        )
        .unwrap();
    let owner = signer.address();
    let salt = U256::ZERO;

    // Manually ABI-encode `createAccount(address owner, uint256 salt)`
    let mut factory_calldata = Vec::with_capacity(4 + 32 + 32);
    factory_calldata.extend_from_slice(&[0x5f, 0xfb, 0xb9, 0xcf]);
    let mut owner_word = [0u8; 32];
    owner_word[12..].copy_from_slice(owner.as_slice());
    factory_calldata.extend_from_slice(&owner_word);
    factory_calldata.extend_from_slice(&salt.to_be_bytes::<32>());

    // initCode = factory ++ factory_calldata
    let mut init_code = Vec::with_capacity(20 + factory_calldata.len());
    init_code.extend_from_slice(cfg.simple_account_factory.as_slice());
    init_code.extend_from_slice(&factory_calldata);
    let init_code_hash = keccak256(&init_code);

    let account = simple_account_address(cfg.simple_account_factory, salt, init_code_hash);
    println!("Owner: {}", owner);
    println!("Counterfactual Account on X Layer: {}", account);
    println!("EntryPoint v0.6: {}", cfg.entry_point_v06);
    println!("Bundler URL: {}", cfg.bundler_url);

    let op = UserOpBuilder::new_v06(
        account,
        U256::ZERO,
        Bytes::from(init_code),
        Bytes::new(),
        U256::from(1000),
        U256::from(100),
    );

    let signed = UserOpBuilder::sign_v06(op, &signer, cfg.entry_point_v06, cfg.chain_id)
        .await
        .unwrap();

    println!(
        "UserOp Hash: {}",
        UserOpBuilder::hash_v06(&signed, cfg.entry_point_v06, cfg.chain_id)
    );
    println!("Signature: 0x{}", hex::encode(&signed.signature));
}
