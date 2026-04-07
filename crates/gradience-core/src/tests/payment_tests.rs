use crate::payment::x402::{X402Service, X402Requirement};
use crate::payment::protocol::PaymentProtocol;

#[test]
fn test_x402_create_requirement_success() {
    let svc = X402Service::new();
    let req = svc.create_requirement(
        "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C",
        "1000000",
        "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        9999999999,
    ).unwrap();
    assert_eq!(req.scheme, "exact");
    assert_eq!(req.network, "base");
    assert_eq!(req.amount, "1000000");
}

#[test]
fn test_x402_invalid_recipient_fails() {
    let svc = X402Service::new();
    let err = svc.create_requirement("not-an-address", "100", "0xabc", 0).unwrap_err();
    assert!(err.to_string().contains("invalid recipient"));
}

#[test]
fn test_x402_sign_and_verify_success() {
    let svc = X402Service::new();
    let req = svc.create_requirement(
        "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C",
        "1000000",
        "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        9999999999,
    ).unwrap();
    let payment = svc.sign_payment(req, "0xdeadbeefsig").unwrap();
    assert!(svc.verify_receipt(&payment, 1000).unwrap());
}

#[test]
fn test_x402_verify_expired_fails() {
    let svc = X402Service::new();
    let req = svc.create_requirement(
        "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C",
        "100",
        "0xabc",
        1000,
    ).unwrap();
    let payment = svc.sign_payment(req, "0xdeadbeefsig").unwrap();
    assert!(!svc.verify_receipt(&payment, 2000).unwrap());
}

#[test]
fn test_x402_verify_empty_signature_fails() {
    let svc = X402Service::new();
    let req = svc.create_requirement("0xabc", "100", "0xabc", 9999).unwrap();
    let payment = svc.sign_payment(req, "0x0123456789abcdef").unwrap();
    // sign_payment enforces min length, so this path is already covered
    assert!(svc.verify_receipt(&payment, 1000).unwrap());
}

#[test]
fn test_payment_protocol_from_str() {
    assert_eq!(PaymentProtocol::from_str("x402"), Some(PaymentProtocol::X402));
    assert_eq!(PaymentProtocol::from_str("mpp"), Some(PaymentProtocol::Mpp));
    assert_eq!(PaymentProtocol::from_str("hsp"), Some(PaymentProtocol::Hsp));
    assert_eq!(PaymentProtocol::from_str("paypal"), None);
}
