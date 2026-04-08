use gradience_core::ows::local_adapter::derive_demo_seed;
use gradience_core::ows::signing::stellar_address_from_secret_key;
use gradience_core::wallet::hd::path_for;

#[test]
fn show_stellar_address() {
    let wallet_id = "367147d7-0215-4b68-b694-38ff9c60a3da";
    let chain = "stellar:testnet";
    let path = path_for(chain, 0);
    let seed = derive_demo_seed(wallet_id, chain, &path);
    let addr = stellar_address_from_secret_key(&seed).unwrap();
    println!("address = {}", addr);
}
