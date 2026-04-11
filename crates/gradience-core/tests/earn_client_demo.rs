use gradience_core::earn::EarnClient;

#[tokio::test]
async fn earn_discover_base_demo() {
    let api_key = std::env::var("LIFI_API_KEY").unwrap_or_default();
    let client = EarnClient::new(api_key);

    match client.discover_vaults_raw(8453, Some(3)).await {
        Ok(vaults) => {
            println!(
                "Found vaults: {}",
                serde_json::to_string_pretty(&vaults).unwrap()
            );
            let arr = vaults
                .get("data")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            assert!(!arr.is_empty(), "expected at least one vault on Base");
        }
        Err(e) => {
            let msg = e.to_string();
            println!("API call failed: {}", msg);
            if msg.contains("rate limit") || msg.contains("429") {
                return;
            }
            panic!("{}", e);
        }
    }
}
