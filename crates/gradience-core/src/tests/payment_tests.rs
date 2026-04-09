use crate::payment::protocol::PaymentProtocol;
use crate::payment::router::{PaymentRouter, PaymentRoutePreference, PaymentRequirement};
use crate::payment::mpp::{MppService, MppPaymentRequest, MppRecipient, BatchTransferPayload, MULTICALL3_ADDRESS};

#[test]
fn test_payment_protocol_from_str() {
    assert_eq!(PaymentProtocol::from_str("mpp"), Some(PaymentProtocol::Mpp));
    assert_eq!(PaymentProtocol::from_str("hsp"), Some(PaymentProtocol::Hsp));
    assert_eq!(PaymentProtocol::from_str("paypal"), None);
    assert_eq!(PaymentProtocol::from_str("unknown"), None);
}

#[tokio::test]
async fn test_payment_router_selects_first_route() {
    let router = PaymentRouter::new(vec![
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
    assert_eq!(route.chain_id, "8453");
}

#[test]
fn test_mpp_service_build_batch_erc20() {
    let svc = MppService::new();
    let req = MppPaymentRequest {
        sender_wallet_id: "wallet-1".into(),
        sender_address: Some("0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C".into()),
        recipients: vec![MppRecipient {
            address: "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C".into(), // EIP-55 checksummed
            amount: "1000000".into(),
        }],
        token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(), // USDC on Base (checksummed)
        chain: "base".into(),
        memo: None,
    };
    let batch = svc.build_batch(&req).unwrap();
    match batch {
        BatchTransferPayload::Evm { to, value, data } => {
            assert_eq!(to, MULTICALL3_ADDRESS);
            assert_eq!(value, "0x0"); // No native value for ERC20
            assert!(data.starts_with("0x1749e1e3")); // aggregate3Value selector
        }
        _ => panic!("expected EVM payload"),
    }
}

#[test]
fn test_mpp_service_build_batch_native() {
    let svc = MppService::new();
    let req = MppPaymentRequest {
        sender_wallet_id: "wallet-1".into(),
        sender_address: Some("0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C".into()),
        recipients: vec![MppRecipient {
            address: "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C".into(), // EIP-55 checksummed address
            amount: "1000000".into(),
        }],
        token_address: "0x0000000000000000000000000000000000000000".into(),
        chain: "base".into(),
        memo: None,
    };
    let batch = svc.build_batch(&req).unwrap();
    match batch {
        BatchTransferPayload::Evm { to, value, data } => {
            assert_eq!(to, MULTICALL3_ADDRESS);
            assert_eq!(value, "0xf4240"); // 1000000 in hex
            assert!(data.starts_with("0x1749e1e3")); // aggregate3Value selector
        }
        _ => panic!("expected EVM payload"),
    }
}

#[test]
fn test_mpp_service_build_batch_empty_recipients_fails() {
    let svc = MppService::new();
    let req = MppPaymentRequest {
        sender_wallet_id: "wallet-1".into(),
        sender_address: Some("0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C".into()),
        recipients: vec![],
        token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(),
        chain: "base".into(),
        memo: None,
    };
    assert!(svc.build_batch(&req).is_err());
}
