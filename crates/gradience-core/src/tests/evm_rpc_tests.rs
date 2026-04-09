use crate::error::GradienceError;
use crate::rpc::evm::EvmRpcClient;

#[tokio::test]
async fn test_evm_get_balance_success() {
    let client = EvmRpcClient::new("eip155:8453", "https://mainnet.base.org").unwrap();
    // Base USDC contract
    let balance: String = client
        .get_balance("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")
        .await
        .unwrap();
    assert!(!balance.is_empty());
    assert!(balance.starts_with("0x"));
}

#[tokio::test]
async fn test_evm_invalid_rpc_url_error() {
    let err = EvmRpcClient::new("eip155:8453", "not-a-url").unwrap_err();
    assert!(matches!(err, GradienceError::Http(_)));
}

#[tokio::test]
async fn test_evm_send_raw_tx_returns_error_for_invalid() {
    let client = EvmRpcClient::new("eip155:8453", "https://mainnet.base.org").unwrap();
    let result: Result<String, GradienceError> = client.send_raw_transaction("0xdeadbeef").await;
    // invalid tx should return JSON-RPC error
    assert!(result.is_err());
}
