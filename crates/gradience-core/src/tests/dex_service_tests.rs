#[tokio::test]
async fn test_build_swap_tx_base_fallback() {
    let svc = crate::dex::service::DexService::new();
    let tx = svc
        .build_swap_tx(
            "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C",
            "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "0x4200000000000000000000000000000000000006",
            "1000000",
            8453,
            50,
        )
        .await
        .unwrap();
    assert!(tx.to.is_some());
    assert!(!tx.data.is_empty());
}

#[tokio::test]
async fn test_build_swap_tx_mock_quote() {
    let svc = crate::dex::service::DexService::new();
    let q = svc
        .get_quote(
            "w1",
            "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            "0x4200000000000000000000000000000000000006",
            "1000000",
            8453,
        )
        .await
        .unwrap();
    assert_eq!(q.from_token, "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");
    assert!(!q.to_amount.is_empty());
}
