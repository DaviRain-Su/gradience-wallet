use crate::payment::protocol::PaymentProtocol;
use crate::payment::router::{PaymentRouter, PaymentRoutePreference, PaymentRequirement};
use crate::payment::mpp::{MppService, MppPaymentRequest, MppRecipient};

#[test]
fn test_payment_protocol_from_str() {
    assert_eq!(PaymentProtocol::from_str("mpp"), Some(PaymentProtocol::Mpp));
    assert_eq!(PaymentProtocol::from_str("hsp"), Some(PaymentProtocol::Hsp));
    assert_eq!(PaymentProtocol::from_str("x402"), None);
    assert_eq!(PaymentProtocol::from_str("paypal"), None);
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
fn test_mpp_service_build_batch() {
    let svc = MppService::new();
    let req = MppPaymentRequest {
        sender_wallet_id: "wallet-1".into(),
        recipients: vec![MppRecipient {
            address: "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C".into(),
            amount: "1000000".into(),
        }],
        token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(),
        chain: "base".into(),
        memo: None,
    };
    let batch = svc.build_batch(&req).unwrap();
    assert!(!batch.is_empty());
}

#[test]
fn test_mpp_service_build_batch_empty_recipients_fails() {
    let svc = MppService::new();
    let req = MppPaymentRequest {
        sender_wallet_id: "wallet-1".into(),
        recipients: vec![],
        token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(),
        chain: "base".into(),
        memo: None,
    };
    assert!(svc.build_batch(&req).is_err());
}
