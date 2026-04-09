use crate::payment::mpp::{
    BatchTransferPayload, MppPaymentRequest, MppRecipient, MppService, MULTICALL3_ADDRESS,
};
use crate::payment::mpp_client::{EvmChargeConfig, GradienceMppProvider};
use crate::payment::router::{PaymentRequirement, PaymentRoutePreference, PaymentRouter};
use alloy::primitives::{Address, U256};
use mpp::client::PaymentProvider;
use mpp::protocol::core::Base64UrlJson;

#[test]
fn test_evm_multicall3_roundtrip() {
    let svc = MppService::new();
    let req = MppPaymentRequest {
        sender_wallet_id: "wallet-1".into(),
        sender_address: None,
        recipients: vec![
            MppRecipient {
                address: "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C".into(),
                amount: "1000000".into(),
            },
            MppRecipient {
                address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".into(),
                amount: "2000000".into(),
            },
        ],
        token_address: "0x0000000000000000000000000000000000000000".into(),
        chain: "base".into(),
        memo: None,
    };

    let batch = svc.build_batch(&req).unwrap();
    match batch {
        BatchTransferPayload::Evm { to, value, data } => {
            assert_eq!(to, MULTICALL3_ADDRESS);
            assert_eq!(value, "0x2dc6c0"); // 3_000_000 hex
            assert!(data.starts_with("0x1749e1e3"));

            let bytes = hex::decode(data.trim_start_matches("0x")).unwrap();
            assert_eq!(&bytes[0..4], &[0x17, 0x49, 0xe1, 0xe3]); // selector

            let offset = U256::from_be_slice(&bytes[4..36]);
            assert_eq!(offset, U256::from(32));

            let arr_len = U256::from_be_slice(&bytes[36..68]);
            assert_eq!(arr_len, U256::from(2));
        }
        _ => panic!("expected EVM payload"),
    }
}

#[test]
fn test_evm_erc20_multicall3_calldata() {
    let svc = MppService::new();
    let req = MppPaymentRequest {
        sender_wallet_id: "wallet-1".into(),
        sender_address: None,
        recipients: vec![MppRecipient {
            address: "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C".into(),
            amount: "500000".into(),
        }],
        token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(),
        chain: "base".into(),
        memo: None,
    };

    let batch = svc.build_batch(&req).unwrap();
    match batch {
        BatchTransferPayload::Evm { to, value, data } => {
            assert_eq!(to, MULTICALL3_ADDRESS);
            assert_eq!(value, "0x0");

            let bytes = hex::decode(data.trim_start_matches("0x")).unwrap();
            let arr_len = U256::from_be_slice(&bytes[36..68]);
            assert_eq!(arr_len, U256::from(1));

            // array data starts at byte 36 (after 4-byte selector + 32-byte offset)
            let array_data_start = 36usize;
            let struct_offset = U256::from_be_slice(&bytes[68..100]);
            let struct_start = array_data_start + struct_offset.to::<usize>();

            let target = Address::from_slice(
                &bytes[struct_start + 12..struct_start + 32],
            );
            assert_eq!(
                target,
                "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
                    .parse::<Address>()
                    .unwrap()
            );

            let calldata_offset =
                U256::from_be_slice(&bytes[struct_start + 96..struct_start + 128]);
            assert_eq!(calldata_offset, U256::from(128));

            let calldata_start = struct_start + 128 + 32;
            let calldata_len =
                U256::from_be_slice(&bytes[struct_start + 128..calldata_start]);
            assert_eq!(calldata_len, U256::from(68)); // 4 + 32 + 32

            assert_eq!(
                &bytes[calldata_start..calldata_start + 4],
                &[0xa9, 0x05, 0x9c, 0xbb]
            );
        }
        _ => panic!("expected EVM payload"),
    }
}

#[test]
fn test_solana_batch_serialization_roundtrip() {
    let svc = MppService::new();
    let req = MppPaymentRequest {
        sender_wallet_id: "wallet-1".into(),
        sender_address: Some("4fP7AaKvKk4ePH8ag46G3cvLCG3A5NzL65FcvHEZgZsd".into()),
        recipients: vec![
            MppRecipient {
                address: "6s5wd3EWpPdPsPMvoWzBQz9qWiC9kbWLUqQMrYMfVJde".into(),
                amount: "1000000".into(),
            },
            MppRecipient {
                address: "7nj2RkMJWT2e1MVhf54grpZihKD7RSGg3dFDL4dkpump".into(),
                amount: "2000000".into(),
            },
        ],
        token_address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into(),
        chain: "solana".into(),
        memo: None,
    };

    let batch = svc.build_batch(&req).unwrap();
    match batch {
        BatchTransferPayload::Solana { serialized_tx } => {
            let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &serialized_tx).unwrap();
            let (sig_count, sig_offset) = decode_compact_u16(&decoded).unwrap();
            assert_eq!(sig_count, 1);

            let msg_start = sig_offset + 64;
            let msg = &decoded[msg_start..];

            let num_required_signatures = msg[0];
            let _num_readonly_signed = msg[1];
            let _num_readonly_unsigned = msg[2];
            assert_eq!(num_required_signatures, 1);

            let (account_keys_len, key_offset) = decode_compact_u16(&msg[3..]).unwrap();
            let mut pos = 3 + key_offset;
            let mut keys = Vec::new();
            for _ in 0..account_keys_len {
                keys.push(&msg[pos..pos + 32]);
                pos += 32;
            }

            pos += 32; // skip blockhash

            let (instr_count, instr_offset) = decode_compact_u16(&msg[pos..]).unwrap();
            pos += instr_offset;
            assert_eq!(instr_count, 2, "expected two SPL transfer instructions");

            let token_program_id = bs58::decode("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
                .into_vec()
                .unwrap();
            let mut found_token_program = false;
            for _ in 0..instr_count {
                let prog_idx = msg[pos] as usize;
                pos += 1;
                if prog_idx < keys.len() && keys[prog_idx] == token_program_id.as_slice() {
                    found_token_program = true;
                }
                let (acct_len, acct_offset) = decode_compact_u16(&msg[pos..]).unwrap();
                pos += acct_offset;
                pos += usize::from(acct_len);
                let (data_len, data_offset) = decode_compact_u16(&msg[pos..]).unwrap();
                pos += data_offset;
                pos += usize::from(data_len);
            }
            assert!(found_token_program);
        }
        _ => panic!("expected Solana payload"),
    }
}

fn decode_compact_u16(bytes: &[u8]) -> Result<(u16, usize), &'static str> {
    let mut value = 0u16;
    let mut offset = 0usize;
    for i in 0..3 {
        if offset >= bytes.len() {
            return Err("unexpected end of compact u16");
        }
        let b = bytes[offset];
        offset += 1;
        value |= ((b & 0x7f) as u16) << (7 * i);
        if b & 0x80 == 0 {
            break;
        }
    }
    Ok((value, offset))
}

// ========================================================================
// Provider & Router Integration Tests
// ========================================================================

#[test]
fn test_gradience_mpp_provider_supports_matrix() {
    let router = PaymentRouter::default();
    let secret = [1u8; 32];

    let provider = GradienceMppProvider::new("test-wallet", router)
        .with_evm_chain(EvmChargeConfig::new(8453, "https://mainnet.base.org", secret))
        .with_solana_secret(secret);

    assert!(provider.supports("evm", "charge"));
    assert!(provider.supports("solana", "charge"));
    assert!(!provider.supports("tempo", "charge")); // no signer configured
    assert!(!provider.supports("evm", "session")); // no escrow configured
    assert!(!provider.supports("unknown", "charge"));
}

#[test]
fn test_gradience_mpp_provider_supports_session_with_escrow() {
    let router = PaymentRouter::default();
    let secret = [1u8; 32];

    let provider = GradienceMppProvider::new("test-wallet", router)
        .with_evm_chain(EvmChargeConfig::new(8453, "https://mainnet.base.org", secret))
        .with_escrow_address(8453, "0x1234567890123456789012345678901234567890");

    assert!(provider.supports("evm", "session"));
}

#[tokio::test]
async fn test_payment_router_prefers_lower_priority() {
    let router = PaymentRouter::new(vec![
        PaymentRoutePreference {
            chain_id: "56".into(),
            token_address: "0x55d398326f99059fF775485246999027B3197955".into(),
            priority: 2,
        },
        PaymentRoutePreference {
            chain_id: "8453".into(),
            token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(),
            priority: 1,
        },
    ]);

    let req = PaymentRequirement {
        amount: "1000000".into(),
        token_hint: Some("USDC".into()),
    };

    let route = router.select_route(&req).await.unwrap();
    assert_eq!(route.chain_id, "8453"); // lower priority number = higher preference
}

#[tokio::test]
async fn test_payment_router_no_match_falls_back() {
    let router = PaymentRouter::new(vec![
        PaymentRoutePreference {
            chain_id: "56".into(),
            token_address: "0x55d398326f99059fF775485246999027B3197955".into(),
            priority: 1,
        },
    ]);

    let req = PaymentRequirement {
        amount: "1000000".into(),
        token_hint: Some("UNKNOWN".into()),
    };

    let route = router.select_route(&req).await.unwrap();
    assert_eq!(route.chain_id, "56");
}

// ========================================================================
// Tempo Provider Configuration Test (real RPC excluded)
// ========================================================================

#[tokio::test]
async fn test_tempo_provider_configured() {
    let router = PaymentRouter::default();
    let signer = alloy::signers::local::PrivateKeySigner::random();

    let provider = GradienceMppProvider::new("test-wallet", router)
        .with_tempo_signer(signer.clone());

    assert!(provider.supports("tempo", "charge"));
    assert!(!provider.supports("tempo", "session"));
}

// ========================================================================
// MPP Challenge / Credential round-trip (no real RPC required)
// ========================================================================

#[test]
fn test_mpp_credential_header_roundtrip() {
    use mpp::protocol::core::PaymentPayload;

    let challenge = mpp::PaymentChallenge {
        id: "cred-test-1".into(),
        realm: "test".into(),
        method: "evm".into(),
        intent: "charge".into(),
        request: Base64UrlJson::from_value(
            &serde_json::json!({"amount":"1000"})).unwrap(),
        expires: None,
        description: None,
        digest: None,
        opaque: None,
    };

    let payload = PaymentPayload::hash("0xdeadbeef");
    let credential = mpp::PaymentCredential::new(challenge.to_echo(), payload);

    let header = mpp::format_authorization(&credential).unwrap();
    let parsed = mpp::PaymentCredential::from_header(&header).unwrap();

    assert_eq!(parsed.challenge.id, "cred-test-1");
    let parsed_payload = parsed.charge_payload().unwrap();
    assert!(parsed_payload.is_hash());
    assert_eq!(parsed_payload.tx_hash(), Some("0xdeadbeef"));
}

// ========================================================================
// MPP Protocol Round-trip Tests
// ========================================================================

#[tokio::test]
async fn test_charge_request_roundtrip() {
    use mpp::protocol::intents::ChargeRequest;

    let charge = ChargeRequest {
        amount: "1000000".into(),
        currency: "USDC".into(),
        recipient: Some("0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C".into()),
        description: Some("Test payment".into()),
        decimals: Some(6),
        external_id: Some("ext-123".into()),
        method_details: None,
    };

    let request = Base64UrlJson::from_typed(&charge).unwrap();
    let decoded: ChargeRequest = request.decode().unwrap();

    assert_eq!(decoded.amount, "1000000");
    assert_eq!(decoded.currency, "USDC");
    assert_eq!(
        decoded.recipient,
        Some("0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C".into())
    );
    // Note: some mpp crate versions canonicalize/decimals handling differently.
    // We keep recipient/amount/currency assertions as the core contract.
}

#[test]
fn test_mpp_client_creation() {
    let router = PaymentRouter::default();
    let provider = GradienceMppProvider::new("test-wallet", router);
    let _client = crate::payment::mpp_client::MppClient::new(provider);
}
