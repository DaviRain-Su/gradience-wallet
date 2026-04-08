use gradience_core::ows::local_adapter::derive_demo_seed;
use gradience_core::ows::signing::eth_address_from_secret_key;
use gradience_core::wallet::hd::path_for;

#[test]
fn show_base_sepolia_address() {
    let wallet_id = "367147d7-0215-4b68-b694-38ff9c60a3da";
    let chain = "eip155:84532";
    let path = path_for(chain, 0);
    let seed = derive_demo_seed(wallet_id, chain, &path);
    let addr = eth_address_from_secret_key(&seed).unwrap();
    let pk = format!("0x{}", hex::encode(&seed));
    println!("address = {}  private_key = {}", addr, pk);
}
