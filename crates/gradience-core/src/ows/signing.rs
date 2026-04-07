use crate::error::{GradienceError, Result};
use secp256k1::{SecretKey, PublicKey};
use sha3::{Keccak256, Digest};
use std::convert::TryInto;

pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().as_slice().try_into().unwrap()
}

pub fn eth_address_from_secret_key(secret: &[u8; 32]) -> Result<String> {
    let sk: SecretKey = SecretKey::from_slice(secret)
        .map_err(|e: secp256k1::Error| GradienceError::Signature(e.to_string()))?;
    let secp = secp256k1::Secp256k1::new();
    let pk = PublicKey::from_secret_key(&secp, &sk);
    let uncompressed = pk.serialize_uncompressed();
    // Remove 0x04 prefix, hash remaining 64 bytes, take last 20 bytes
    let hash = keccak256(&uncompressed[1..]);
    let addr = format!("0x{}", hex::encode(&hash[12..]));
    Ok(addr)
}

pub fn sign_eth_transaction(
    secret: &[u8; 32],
    nonce: u64,
    gas_price: u128,
    gas_limit: u64,
    to: &str,
    value: u128,
    data: &[u8],
    chain_id: u64,
) -> Result<Vec<u8>> {
    let sk: SecretKey = SecretKey::from_slice(secret)
        .map_err(|e: secp256k1::Error| GradienceError::Signature(e.to_string()))?;
    let secp = secp256k1::Secp256k1::new();
    let expected_pk = PublicKey::from_secret_key(&secp, &sk);

    let to_bytes = if to.is_empty() || to == "0x" {
        vec![]
    } else {
        hex::decode(to.trim_start_matches("0x")).unwrap_or_default()
    };

    // RLP encode unsigned tx with chain_id for EIP-155
    let mut rlp = rlp::RlpStream::new_list(9);
    rlp.append(&nonce);
    rlp.append(&gas_price);
    rlp.append(&gas_limit);
    rlp.append(&to_bytes);
    rlp.append(&value);
    rlp.append(&data);
    rlp.append(&chain_id);
    rlp.append(&0u8);
    rlp.append(&0u8);
    let encoded = rlp.out();

    let tx_hash = keccak256(&encoded);
    let msg = secp256k1::Message::from_digest_slice(&tx_hash)
        .map_err(|e: secp256k1::Error| GradienceError::Signature(e.to_string()))?;
    let sig = secp.sign_ecdsa(&msg, &sk);
    let raw_sig = sig.serialize_compact();
    let r = &raw_sig[0..32];
    let s = &raw_sig[32..64];

    // brute-force recovery id 0 or 1
    let mut recovery_id: u64 = 0;
    for i in 0..2u8 {
        if let Ok(pk) = secp.recover_ecdsa(&msg, &secp256k1::ecdsa::RecoverableSignature::from_compact(&raw_sig, secp256k1::ecdsa::RecoveryId::from_i32(i as i32).unwrap()).unwrap()) {
            if pk == expected_pk {
                recovery_id = i as u64;
                break;
            }
        }
    }

    let v: u64 = recovery_id.wrapping_add(chain_id * 2 + 35);

    let mut signed_rlp = rlp::RlpStream::new_list(9);
    signed_rlp.append(&nonce);
    signed_rlp.append(&gas_price);
    signed_rlp.append(&gas_limit);
    signed_rlp.append(&to_bytes);
    signed_rlp.append(&value);
    signed_rlp.append(&data);
    signed_rlp.append(&v);
    signed_rlp.append(&r);
    signed_rlp.append(&s);

    Ok(signed_rlp.out().into())
}
