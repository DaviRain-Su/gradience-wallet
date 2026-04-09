use gradience_core::payment::state_channel::sign_state_update;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        eprintln!(
            "Usage: generate_state_channel_sig <secret_hex> <channel_id_hex> <nonce> <amount>"
        );
        std::process::exit(1);
    }

    let secret_hex = args[1].trim_start_matches("0x");
    let mut secret = [0u8; 32];
    hex::decode_to_slice(secret_hex, &mut secret).expect("invalid secret hex");

    let channel_id_hex = args[2].trim_start_matches("0x");
    let mut channel_id = [0u8; 32];
    hex::decode_to_slice(channel_id_hex, &mut channel_id).expect("invalid channel_id hex");

    let nonce: u64 = args[3].parse().expect("invalid nonce");
    let amount: u128 = args[4].parse().expect("invalid amount");

    let sig = sign_state_update(&secret, &channel_id, nonce, amount).expect("sign failed");
    println!("0x{}", hex::encode(sig));
}
