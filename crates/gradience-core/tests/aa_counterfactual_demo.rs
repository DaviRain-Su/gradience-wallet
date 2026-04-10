use alloy::primitives::{keccak256, Address, Bytes, U256, address, b256};
use alloy::signers::local::PrivateKeySigner;
use gradience_core::aa::factory::simple_account_address;
use gradience_core::aa::user_op::UserOpBuilder;

#[tokio::test]
async fn demo_counterfactual_userop() {
    // Deterministic owner key for demo
    let signer =
        PrivateKeySigner::from_bytes(
            &b256!(
                "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            )
        )
        .unwrap();
    let owner = signer.address();

    // Base Sepolia SimpleAccountFactory (placeholder)
    let factory: Address = address!("9406Cc6185a346906296840746125a0E44976454");
    let salt = U256::ZERO;

    // Manually ABI-encode `createAccount(address owner, uint256 salt)`
    // selector = keccak256("createAccount(address,uint256)")[0:4] = 0x5fbfb9cf
    let mut factory_calldata = Vec::with_capacity(4 + 32 + 32);
    factory_calldata.extend_from_slice(&[0x5f, 0xfb, 0xb9, 0xcf]);
    let mut owner_word = [0u8; 32];
    owner_word[12..].copy_from_slice(owner.as_slice());
    factory_calldata.extend_from_slice(&owner_word);
    factory_calldata.extend_from_slice(&salt.to_be_bytes::<32>());

    // initCode = factory ++ factory_calldata
    let mut init_code = Vec::with_capacity(20 + factory_calldata.len());
    init_code.extend_from_slice(factory.as_slice());
    init_code.extend_from_slice(&factory_calldata);
    let init_code_hash = keccak256(&init_code);

    let account = simple_account_address(factory, salt, init_code_hash);
    println!("Owner: {}", owner);
    println!("Counterfactual Account: {}", account);

    // EntryPoint v0.6
    let entry_point: Address = address!("5FF137D4b0FDCD49DcA30c7CF57E578a026d2789");

    let op = UserOpBuilder::new_v06(
        account,
        U256::ZERO,
        Bytes::from(init_code),
        Bytes::new(),
        U256::from(1000),
        U256::from(100),
    );

    let signed = UserOpBuilder::sign_v06(op, &signer, entry_point, 84532)
        .await
        .unwrap();

    println!(
        "UserOp Hash: {}",
        UserOpBuilder::hash_v06(&signed, entry_point, 84532)
    );
    println!("Signature: 0x{}", hex::encode(&signed.signature));
}
