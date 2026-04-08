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

// ========================================================================
// Solana minimal transaction helpers (no solana-sdk dependency)
// ========================================================================

fn encode_compact_u16(value: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut val = value;
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        bytes.push(byte);
        if val == 0 {
            break;
        }
    }
    bytes
}

/// Build an unsigned Solana legacy transfer transaction envelope.
/// The envelope format is:
///   [compact_u16(num_signatures)] [64-byte placeholder per signature] [message]
/// This can be fed directly to `ows_lib::sign_transaction` for Solana chains;
/// the SolanaSigner will extract the message, sign it, and splice the signature back in.
pub fn build_solana_transfer_tx(
    from: &str,
    to: &str,
    lamports: u64,
    recent_blockhash_b58: &str,
) -> Result<Vec<u8>> {
    let from_pubkey = bs58::decode(from)
        .into_vec()
        .map_err(|e| GradienceError::Validation(format!("invalid from address: {}", e)))?;
    if from_pubkey.len() != 32 {
        return Err(GradienceError::Validation("from address must be 32 bytes".into()));
    }

    let to_pubkey = bs58::decode(to)
        .into_vec()
        .map_err(|e| GradienceError::Validation(format!("invalid to address: {}", e)))?;
    if to_pubkey.len() != 32 {
        return Err(GradienceError::Validation("to address must be 32 bytes".into()));
    }

    let blockhash = bs58::decode(recent_blockhash_b58)
        .into_vec()
        .map_err(|e| GradienceError::Validation(format!("invalid blockhash: {}", e)))?;
    if blockhash.len() != 32 {
        return Err(GradienceError::Validation("blockhash must be 32 bytes".into()));
    }

    // System program ID (all zeros)
    let system_program: [u8; 32] = [0; 32];

    // Account keys: from (signer+writable), to (writable), system_program (readonly)
    let account_keys: Vec<[u8; 32]> = vec![
        from_pubkey.as_slice().try_into().unwrap(),
        to_pubkey.as_slice().try_into().unwrap(),
        system_program,
    ];

    let num_required_signatures: u8 = 1;
    let num_readonly_signed_accounts: u8 = 0;
    let num_readonly_unsigned_accounts: u8 = 1; // system_program

    // Instruction data: Transfer discriminant (u32 LE = 2) + lamports (u64 LE)
    let mut instruction_data = vec![0u8; 12];
    instruction_data[0..4].copy_from_slice(&2u32.to_le_bytes());
    instruction_data[4..12].copy_from_slice(&lamports.to_le_bytes());

    // Compiled instruction for system program
    let mut instruction = vec![];
    instruction.push(2u8); // program_id_index = system_program
    instruction.extend_from_slice(&encode_compact_u16(2));
    instruction.push(0u8); // from index
    instruction.push(1u8); // to index
    instruction.extend_from_slice(&encode_compact_u16(instruction_data.len() as u16));
    instruction.extend_from_slice(&instruction_data);

    // Build message
    let mut message = vec![];
    message.push(num_required_signatures);
    message.push(num_readonly_signed_accounts);
    message.push(num_readonly_unsigned_accounts);
    message.extend_from_slice(&encode_compact_u16(account_keys.len() as u16));
    for key in &account_keys {
        message.extend_from_slice(key);
    }
    message.extend_from_slice(&blockhash);
    message.extend_from_slice(&encode_compact_u16(1)); // 1 instruction
    message.extend_from_slice(&instruction);

    // Envelope: 1 signature placeholder + message
    let mut tx = encode_compact_u16(1);
    tx.extend_from_slice(&[0u8; 64]);
    tx.extend_from_slice(&message);

    Ok(tx)
}
