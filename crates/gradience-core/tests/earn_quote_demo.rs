use gradience_core::earn::EarnClient;

#[tokio::test]
async fn earn_quote_base_demo() {
    let api_key = std::env::var("LIFI_API_KEY").unwrap_or_default();
    let client = EarnClient::new(api_key);

    // Base Sepolia USDC -> yo-protocol USDC vault on Base
    let from_address = "0x742d35cc6634c0532925a3b844bc9e7595f2bd0c";
    let to_address = from_address;
    let from_token = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"; // USDC on Base
    let to_token = "0x0000000f2eB9f69274678c76222B35eEc7588a65";   // yo-protocol USDC vault
    let amount = "1000000"; // 1 USDC

    match client
        .quote_deposit(8453, 8453, from_token, to_token, from_address, to_address, amount)
        .await
    {
        Ok(quote) => {
            println!("Quote response:\n{}", serde_json::to_string_pretty(&quote).unwrap());
        }
        Err(e) => {
            println!("Quote failed: {}", e);
            panic!("{}", e);
        }
    }
}
