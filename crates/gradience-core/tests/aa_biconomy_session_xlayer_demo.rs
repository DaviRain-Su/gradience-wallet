use alloy::primitives::{Address, Bytes, U256, address, b256};
use alloy::signers::Signer;
use alloy::signers::local::PrivateKeySigner;
use gradience_core::aa::biconomy::BiconomyAccount;
use gradience_core::aa::biconomy_session::BiconomySession;
use gradience_core::aa::user_op::UserOpBuilder;

#[tokio::test]
async fn demo_biconomy_session_key_xlayer() {
    // X Layer Mainnet config
    let rpc_url = "https://rpc.xlayer.tech";
    let factory: Address = address!("000000a56Aaca3e9a4C479ea6b6CD0DbcB6634F5");
    let ecdsa_module: Address = address!("0000001c5b32F37F5beA87BDD5374eB2aC54eA8e");
    let skm_module: Address = address!("000002FbFfedd9B33F4E7156F2DE8D48945E7489");
    let entry_point: Address = address!("5FF137D4b0FDCD49DcA30c7CF57E578a026d2789");
    let chain_id = 196u64;

    // Owner (master key)
    let owner_signer = PrivateKeySigner::from_bytes(
        &b256!("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
    ).unwrap();
    let owner = owner_signer.address();

    // Session key (agent key)
    let session_signer = PrivateKeySigner::from_bytes(
        &b256!("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
    ).unwrap();
    let session_key = session_signer.address();

    let index = U256::ZERO;

    // 1. Query SmartAccount address
    let account = BiconomyAccount::get_counterfactual_address(
        rpc_url, factory, ecdsa_module, owner, index,
    ).await.unwrap();
    println!("Owner: {}", owner);
    println!("Session Key: {}", session_key);
    println!("Biconomy SmartAccount: {}", account);

    // 2. Build session key data (ERC20 module format for demo)
    let token: Address = address!("74b7F16337b8972027F6196A1aE9b4dDc0C42b50");
    let recipient: Address = address!("909E30bdBCb728131E3F8d17150eaE740C904649");
    let max_amount = U256::from(1_000_000);
    let session_key_data = BiconomySession::encode_erc20_session_key_data(
        session_key, token, recipient, max_amount,
    );

    // 3. Compute merkle leaf and root (single leaf -> root == leaf)
    let valid_until = 0u64;
    let valid_after = 0u64;
    let erc20_svm: Address = address!("000000D50C68705bd6897B2d17c7de32FB519fDA");
    let leaf = BiconomySession::leaf_hash(valid_until, valid_after, erc20_svm, &session_key_data);
    let merkle_root = BiconomySession::single_leaf_merkle_root(&leaf);
    println!("Session Leaf: {}", leaf);
    println!("Merkle Root: {}", merkle_root);

    // ======================== PHASE 1: setMerkleRoot (owner signed) ========================
    let set_root_call = BiconomySession::build_set_merkle_root_call_data(merkle_root);
    let init_code = BiconomyAccount::build_init_code(factory, ecdsa_module, owner, index);

    let mut op_set_root = UserOpBuilder::new_v06(
        account,
        U256::ZERO,
        init_code.clone(),
        set_root_call,
        U256::from(1000),
        U256::from(100),
    );

    BiconomyAccount::sign_user_op(
        &mut op_set_root,
        &owner_signer,
        entry_point,
        chain_id,
        ecdsa_module,
    ).await.unwrap();

    println!("\n--- Phase 1: setMerkleRoot UserOp ---");
    println!("UserOp Hash: {}", UserOpBuilder::hash_v06(&op_set_root, entry_point, chain_id));
    println!("Signature length: {} bytes", op_set_root.signature.len());

    // ======================== PHASE 2: execute with session key ========================
    // Demo call: simple execute to recipient with 0 value and empty data
    let exec_call = BiconomyAccount::build_execute_call_data(
        recipient, U256::ZERO, Bytes::new(),
    );

    let mut op_exec = UserOpBuilder::new_v06(
        account,
        U256::from(1), // nonce increments after deployment
        Bytes::new(),  // no initCode for already-deployed account
        exec_call,
        U256::from(1000),
        U256::from(100),
    );

    let user_op_hash = UserOpBuilder::hash_v06(&op_exec, entry_point, chain_id);

    // Session key signs the userOpHash
    let session_sig = session_signer.sign_hash(&user_op_hash).await.unwrap();
    let session_sig_bytes = session_sig.as_bytes();

    let module_sig = BiconomySession::build_module_signature(
        valid_until,
        valid_after,
        erc20_svm,
        session_key_data,
        vec![], // empty merkle proof for single-leaf tree
        &session_sig_bytes[..],
    );

    op_exec.signature = BiconomySession::build_user_op_signature(module_sig.clone(), skm_module);

    println!("\n--- Phase 2: Session-enabled Execute UserOp ---");
    println!("UserOp Hash: {}", user_op_hash);
    println!("Module Signature length: {} bytes", module_sig.len());
    println!("Final Signature length: {} bytes", op_exec.signature.len());
}
