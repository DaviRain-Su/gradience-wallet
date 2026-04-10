use alloy::primitives::b256;
use alloy::signers::local::PrivateKeySigner;
use gradience_core::aa::particle::{AccountConfig, ParticleClient};

/// Demo: query Biconomy V2 smart account address on X Layer via Particle Enhanced API.
///
/// Requires `PARTICLE_PROJECT_ID` and `PARTICLE_CLIENT_KEY` env vars.
/// If missing, the test prints a skip message and returns OK.
#[tokio::test]
async fn demo_particle_get_smart_account_xlayer() {
    let project_id = std::env::var("PARTICLE_PROJECT_ID").unwrap_or_default();
    let client_key = std::env::var("PARTICLE_CLIENT_KEY").unwrap_or_default();

    if project_id.is_empty() || client_key.is_empty() {
        println!("SKIP: set PARTICLE_PROJECT_ID and PARTICLE_CLIENT_KEY to run this demo.");
        return;
    }

    // Deterministic owner for demo
    let signer =
        PrivateKeySigner::from_bytes(
            &b256!(
                "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            )
        )
        .unwrap();
    let owner = signer.address();

    let client = ParticleClient::new(&project_id, &client_key);
    let account_config = AccountConfig {
        name: "BICONOMY".into(),
        version: "2.0.0".into(),
        owner_address: owner.to_string(),
        biconomy_api_key: None,
    };

    match client.get_smart_account(196, vec![account_config]).await {
        Ok(accounts) => {
            println!("Owner: {}", owner);
            for (i, acc) in accounts.iter().enumerate() {
                println!(
                    "SmartAccount #{} on X Layer: {}",
                    i, acc.smart_account_address
                );
            }
        }
        Err(e) => {
            println!("Particle getSmartAccount failed: {}", e);
        }
    }
}
