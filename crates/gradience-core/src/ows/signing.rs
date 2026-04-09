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

pub fn stellar_address_from_secret_key(secret: &[u8; 32]) -> Result<String> {
    use ed25519_dalek::{SigningKey, VerifyingKey};
    let signing_key = SigningKey::from_bytes(secret);
    let verifying_key: VerifyingKey = signing_key.verifying_key();
    let pk = stellar_strkey::ed25519::PublicKey(verifying_key.to_bytes());
    Ok(pk.to_string())
}

pub fn stellar_secret_from_seed(seed: &[u8; 32]) -> String {
    let sk = stellar_strkey::ed25519::PrivateKey(*seed);
    sk.to_string()
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

fn decode_compact_u16(data: &[u8]) -> Result<(u16, usize)> {
    let mut value: u16 = 0;
    let mut shift = 0;
    for (i, &byte) in data.iter().enumerate() {
        let bits = (byte & 0x7F) as u16;
        value |= bits << shift;
        if byte & 0x80 == 0 {
            return Ok((value, i + 1));
        }
        shift += 7;
        if shift > 14 {
            return Err(GradienceError::Validation("compact u16 overflow".into()));
        }
    }
    Err(GradienceError::Validation("incomplete compact u16".into()))
}

/// Sign a Solana legacy transaction envelope built by `build_solana_transfer_tx`.
/// Replaces the 64-byte signature placeholder with an ed25519 signature over the message.
pub fn sign_solana_transaction(mut tx_bytes: Vec<u8>, secret: &[u8; 32]) -> Result<Vec<u8>> {
    use ed25519_dalek::{Signer, SigningKey};

    let (num_sigs, header_len) = decode_compact_u16(&tx_bytes)?;
    if num_sigs != 1 {
        return Err(GradienceError::Validation("expected 1 signature placeholder".into()));
    }
    let msg_start = header_len + 64;
    if tx_bytes.len() < msg_start {
        return Err(GradienceError::Validation("solana tx too short".into()));
    }
    let message = &tx_bytes[msg_start..];

    let signing_key = SigningKey::from_bytes(secret);
    let signature = signing_key.sign(message);

    tx_bytes[header_len..header_len + 64].copy_from_slice(&signature.to_bytes());
    Ok(tx_bytes)
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

// ========================================================================
// Solana SPL Token + Staking helpers
// ========================================================================

fn decode_pubkey(b58: &str) -> Result<[u8; 32]> {
    let bytes = bs58::decode(b58)
        .into_vec()
        .map_err(|e| GradienceError::Validation(format!("invalid base58 pubkey: {}", e)))?;
    if bytes.len() != 32 {
        return Err(GradienceError::Validation("pubkey must be 32 bytes".into()));
    }
    Ok(bytes.try_into().unwrap())
}

fn encode_pubkey(bytes: &[u8; 32]) -> String {
    bs58::encode(bytes).into_string()
}

/// Check if 32 bytes represent a point on the ed25519 curve.
fn is_on_curve(bytes: &[u8; 32]) -> bool {
    ed25519_dalek::VerifyingKey::from_bytes(bytes).is_ok()
}

/// Solana `create_program_address` using SHA256 (ring) + ed25519 on-curve check.
fn create_program_address(seeds: &[&[u8]], program_id: &[u8; 32]) -> Result<[u8; 32]> {
    use ring::digest::{Context, SHA256};
    let mut ctx = Context::new(&SHA256);
    for seed in seeds {
        ctx.update(seed);
    }
    ctx.update(program_id);
    ctx.update(b"ProgramDerivedAddress");
    let hash = ctx.finish();
    let bytes: [u8; 32] = hash.as_ref().try_into().unwrap();
    if is_on_curve(&bytes) {
        return Err(GradienceError::Validation("invalid PDA: on curve".into()));
    }
    Ok(bytes)
}

/// Solana `find_program_address`: tries bump seeds 255..0.
fn find_program_address(seeds: &[&[u8]], program_id: &[u8; 32]) -> Result<([u8; 32], u8)> {
    for bump in (0..=255).rev() {
        let bump_seed = [bump];
        let mut seeds_with_bump = seeds.to_vec();
        seeds_with_bump.push(&bump_seed);
        if let Ok(addr) = create_program_address(&seeds_with_bump, program_id) {
            return Ok((addr, bump));
        }
    }
    Err(GradienceError::Validation("unable to find valid PDA".into()))
}

/// Derive the Associated Token Account (ATA) for a wallet + mint.
pub fn find_associated_token_address(wallet_b58: &str, mint_b58: &str) -> Result<String> {
    let wallet = decode_pubkey(wallet_b58)?;
    let mint = decode_pubkey(mint_b58)?;
    let ata_program = decode_pubkey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")?;
    let token_program = decode_pubkey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")?;
    let seeds: Vec<&[u8]> = vec![&wallet, &token_program, &mint];
    let (addr, _) = find_program_address(&seeds, &ata_program)?;
    Ok(encode_pubkey(&addr))
}

/// Build a Solana Legacy Message and wrap it in a transaction envelope.
/// `instructions` are (program_id_index, accounts: Vec<(index, is_signer, is_writable)>, data).
fn _build_solana_message(
    _payer: &[u8; 32],
    account_keys: Vec<[u8; 32]>,
    recent_blockhash: &[u8; 32],
    instructions: Vec<(u8, Vec<(u8, bool, bool)>, Vec<u8>)>,
) -> Vec<u8> {
    let num_required_signatures: u8 = 1;
    let num_readonly_signed_accounts: u8 = 0;

    // Count readonly unsigned accounts (everything after the last writable account, or more precisely all non-signer non-writable)
    let readonly_unsigned: u8 = account_keys
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 0) // exclude payer (signer)
        .filter(|(i, _)| {
            // Determine if any instruction references this key as writable
            let idx = *i as u8;
            !instructions.iter().any(|(_, accts, _)| {
                accts.iter().any(|(a, _, w)| *a == idx && *w)
            })
        })
        .count() as u8;

    let mut message = vec![];
    message.push(num_required_signatures);
    message.push(num_readonly_signed_accounts);
    message.push(readonly_unsigned);
    message.extend_from_slice(&encode_compact_u16(account_keys.len() as u16));
    for key in &account_keys {
        message.extend_from_slice(key);
    }
    message.extend_from_slice(recent_blockhash);
    message.extend_from_slice(&encode_compact_u16(instructions.len() as u16));

    for (prog_idx, accounts, data) in instructions {
        message.push(prog_idx);
        message.extend_from_slice(&encode_compact_u16(accounts.len() as u16));
        for (idx, is_signer, is_writable) in accounts {
            let mut meta = 0u8;
            if is_signer {
                meta |= 0x80;
            }
            if is_writable {
                meta |= 0x40;
            }
            message.push(meta | idx);
        }
        message.extend_from_slice(&encode_compact_u16(data.len() as u16));
        message.extend_from_slice(&data);
    }

    let mut tx = encode_compact_u16(1);
    tx.extend_from_slice(&[0u8; 64]);
    tx.extend_from_slice(&message);
    tx
}

/// Build a complete SPL Token transfer transaction.
/// If `create_recipient_ata` is true, includes the `CreateAssociatedTokenAccount` instruction.
pub fn build_spl_transfer_tx(
    from_wallet_b58: &str,
    to_wallet_b58: &str,
    mint_b58: &str,
    amount: u64,
    decimals: u8,
    recent_blockhash_b58: &str,
    create_recipient_ata: bool,
) -> Result<Vec<u8>> {
    let from_wallet = decode_pubkey(from_wallet_b58)?;
    let to_wallet = decode_pubkey(to_wallet_b58)?;
    let mint = decode_pubkey(mint_b58)?;
    let token_program = decode_pubkey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")?;
    let ata_program = decode_pubkey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")?;
    let system_program = decode_pubkey("11111111111111111111111111111111")?;
    let from_ata = decode_pubkey(&find_associated_token_address(from_wallet_b58, mint_b58)?)?;
    let to_ata = decode_pubkey(&find_associated_token_address(to_wallet_b58, mint_b58)?)?;
    let blockhash = decode_pubkey(recent_blockhash_b58)?;

    // Assemble unique account keys in required order:
    // 0: from_wallet (signer, writable)
    // 1: from_ata (writable)
    // 2: to_ata (writable)
    // 3: to_wallet (readonly)
    // 4: mint (readonly)
    // 5: token_program (readonly)
    // 6: system_program (readonly, optional)
    // 7: ata_program (readonly, optional)
    let mut keys: Vec<[u8; 32]> = vec![
        from_wallet, from_ata, to_ata, to_wallet, mint, token_program,
    ];
    if create_recipient_ata {
        keys.push(system_program);
        keys.push(ata_program);
    }

    let mut instructions: Vec<(u8, Vec<(u8, bool, bool)>, Vec<u8>)> = vec![];

    if create_recipient_ata {
        // CreateAssociatedTokenAccount: no data
        // accounts: [payer(0), ata(2), owner(3), mint(4), system_program(6), token_program(5)]
        // Note: keys vector index carefully
        let create_accounts = vec![
            (0, true, true),   // payer
            (2, false, true),  // ata
            (3, false, false), // owner
            (4, false, false), // mint
            (6, false, false), // system_program
            (5, false, false), // token_program
        ];
        let ata_prog_idx = keys.iter().position(|k| k == &ata_program).unwrap() as u8;
        instructions.push((ata_prog_idx, create_accounts, vec![]));
    }

    // TransferChecked data: [12, amount(le64), decimals]
    let mut transfer_data = vec![12u8];
    transfer_data.extend_from_slice(&amount.to_le_bytes());
    transfer_data.push(decimals);

    let transfer_accounts = vec![
        (1, false, true),  // source (from_ata)
        (4, false, false), // mint
        (2, false, true),  // destination (to_ata)
        (0, true, false),  // owner (from_wallet)
    ];
    let token_prog_idx = keys.iter().position(|k| k == &token_program).unwrap() as u8;
    instructions.push((token_prog_idx, transfer_accounts, transfer_data));

    // Recompute readonly count for message header correctly.
    // Payer (index 0) is signer+writable, don't count.
    // Everything else that is NOT writable is readonly unsigned.
    let mut readonly_unsigned = 0u8;
    for (i, _) in keys.iter().enumerate().skip(1) {
        let idx = i as u8;
        let is_writable = instructions.iter().any(|(_, accts, _)| {
            accts.iter().any(|(a, _, w)| *a == idx && *w)
        });
        if !is_writable {
            readonly_unsigned += 1;
        }
    }

    let mut message = vec![];
    message.push(1u8); // num_required_signatures
    message.push(0u8); // num_readonly_signed_accounts
    message.push(readonly_unsigned);
    message.extend_from_slice(&encode_compact_u16(keys.len() as u16));
    for key in &keys {
        message.extend_from_slice(key);
    }
    message.extend_from_slice(&blockhash);
    message.extend_from_slice(&encode_compact_u16(instructions.len() as u16));

    for (prog_idx, accounts, data) in instructions {
        message.push(prog_idx);
        message.extend_from_slice(&encode_compact_u16(accounts.len() as u16));
        for (idx, is_signer, is_writable) in accounts {
            let mut meta = 0u8;
            if is_signer {
                meta |= 0x80;
            }
            if is_writable {
                meta |= 0x40;
            }
            message.push(meta | idx);
        }
        message.extend_from_slice(&encode_compact_u16(data.len() as u16));
        message.extend_from_slice(&data);
    }

    let mut tx = encode_compact_u16(1);
    tx.extend_from_slice(&[0u8; 64]);
    tx.extend_from_slice(&message);
    Ok(tx)
}

/// Build a Solana DelegateStake transaction.
/// The caller must ensure `stake_account` exists and is initialized.
pub fn build_delegate_stake_tx(
    stake_account_b58: &str,
    vote_account_b58: &str,
    authorized_staker_b58: &str,
    recent_blockhash_b58: &str,
) -> Result<Vec<u8>> {
    let stake_account = decode_pubkey(stake_account_b58)?;
    let vote_account = decode_pubkey(vote_account_b58)?;
    let authorized_staker = decode_pubkey(authorized_staker_b58)?;
    let clock = decode_pubkey("SysvarC1ock111111111111111111111111111111111")?;
    let stake_history = decode_pubkey("SysvarStakeHistory1111111111111111111111111")?;
    let stake_config = decode_pubkey("StakeConfig11111111111111111111111111111111")?;
    let stake_program = decode_pubkey("Stake11111111111111111111111111111111111111")?;
    let blockhash = decode_pubkey(recent_blockhash_b58)?;

    let account_keys = vec![
        stake_account,
        authorized_staker,
        vote_account,
        clock,
        stake_history,
        stake_config,
        stake_program,
    ];

    let num_required_signatures = 1u8;
    let num_readonly_signed_accounts = 1u8;
    let num_readonly_unsigned_accounts = 5u8;

    let mut message = vec![];
    message.push(num_required_signatures);
    message.push(num_readonly_signed_accounts);
    message.push(num_readonly_unsigned_accounts);
    message.extend_from_slice(&encode_compact_u16(account_keys.len() as u16));
    for key in &account_keys {
        message.extend_from_slice(key);
    }
    message.extend_from_slice(&blockhash);

    let instruction_accounts = vec![
        (0u8, false, true),  // stake_account (writable)
        (2u8, false, false), // vote_account
        (3u8, false, false), // clock
        (4u8, false, false), // stake_history
        (5u8, false, false), // stake_config
        (1u8, true, false),  // authorized_staker (signer)
    ];
    let prog_idx = 6u8;
    message.extend_from_slice(&encode_compact_u16(1));
    message.push(prog_idx);
    message.extend_from_slice(&encode_compact_u16(instruction_accounts.len() as u16));
    for (idx, is_signer, is_writable) in instruction_accounts {
        let mut meta = 0u8;
        if is_signer { meta |= 0x80; }
        if is_writable { meta |= 0x40; }
        message.push(meta | idx);
    }
    message.extend_from_slice(&encode_compact_u16(1));
    message.push(2u8); // DelegateStake discriminant

    let mut tx = encode_compact_u16(1);
    tx.extend_from_slice(&[0u8; 64]);
    tx.extend_from_slice(&message);
    Ok(tx)
}

/// Build a Solana DeactivateStake transaction.
pub fn build_deactivate_stake_tx(
    stake_account_b58: &str,
    authorized_staker_b58: &str,
    recent_blockhash_b58: &str,
) -> Result<Vec<u8>> {
    let stake_account = decode_pubkey(stake_account_b58)?;
    let authorized_staker = decode_pubkey(authorized_staker_b58)?;
    let clock = decode_pubkey("SysvarC1ock111111111111111111111111111111111")?;
    let stake_program = decode_pubkey("Stake11111111111111111111111111111111111111")?;
    let blockhash = decode_pubkey(recent_blockhash_b58)?;

    let account_keys = vec![
        stake_account,
        authorized_staker,
        clock,
        stake_program,
    ];

    let num_required_signatures = 1u8;
    let num_readonly_signed_accounts = 1u8;
    let num_readonly_unsigned_accounts = 2u8;

    let mut message = vec![];
    message.push(num_required_signatures);
    message.push(num_readonly_signed_accounts);
    message.push(num_readonly_unsigned_accounts);
    message.extend_from_slice(&encode_compact_u16(account_keys.len() as u16));
    for key in &account_keys {
        message.extend_from_slice(key);
    }
    message.extend_from_slice(&blockhash);

    let instruction_accounts = vec![
        (0u8, false, true),  // stake_account (writable)
        (1u8, true, false),  // authorized_staker (signer)
        (2u8, false, false), // clock
    ];
    let prog_idx = 3u8;
    message.extend_from_slice(&encode_compact_u16(1));
    message.push(prog_idx);
    message.extend_from_slice(&encode_compact_u16(instruction_accounts.len() as u16));
    for (idx, is_signer, is_writable) in instruction_accounts {
        let mut meta = 0u8;
        if is_signer { meta |= 0x80; }
        if is_writable { meta |= 0x40; }
        message.push(meta | idx);
    }
    message.extend_from_slice(&encode_compact_u16(1));
    message.push(5u8); // DeactivateStake discriminant

    let mut tx = encode_compact_u16(1);
    tx.extend_from_slice(&[0u8; 64]);
    tx.extend_from_slice(&message);
    Ok(tx)
}

// ==================== TON helpers ====================

use std::sync::Arc;
use ton_contracts::wallet::{KeyPair, Wallet, v4r2::V4R2};
use tlb_ton::{
    action::SendMsgAction,
    message::Message,
    ser::{CellBuilder, CellSerialize, CellSerializeExt},
    bits::NBits,
    currency::Grams,
    Ref, EitherInlineOrRef,
    BoC,
    BagOfCellsArgs,
    MsgAddress,
};
use num_bigint::BigUint;

/// Alias for tlb cell builder error.
pub type TonCellError = tlb_ton::ser::CellBuilderError;

pub fn ton_secret_from_seed(seed: &[u8]) -> [u8; 32] {
    let mut hasher = sha2::Sha512::new();
    hasher.update(seed);
    hasher.update(b"TON");
    let hash = hasher.finalize();
    let mut secret = [0u8; 32];
    secret.copy_from_slice(&hash[..32]);
    secret
}

pub fn ton_address_from_seed(seed: &[u8]) -> Result<String> {
    let secret = ton_secret_from_seed(seed);
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret);
    let pubkey = signing_key.verifying_key().to_bytes();

    // nacl/libsodium secret key format: [32 bytes seed][32 bytes pubkey]
    let mut nacl_secret = [0u8; 64];
    nacl_secret[..32].copy_from_slice(&secret);
    nacl_secret[32..].copy_from_slice(&pubkey);

    let keypair = KeyPair::new(nacl_secret, pubkey);
    let wallet = Wallet::<V4R2>::derive_default(keypair)
        .map_err(|e| GradienceError::Validation(format!("ton wallet derive failed: {}", e)))?;
    Ok(wallet.address().to_base64_std())
}

pub fn build_ton_transfer_tx(
    seed: &[u8],
    to: &str,
    amount_nanoton: u64,
    seqno: u32,
) -> Result<Vec<u8>> {
    let secret = ton_secret_from_seed(seed);
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret);
    let pubkey = signing_key.verifying_key().to_bytes();

    let mut nacl_secret = [0u8; 64];
    nacl_secret[..32].copy_from_slice(&secret);
    nacl_secret[32..].copy_from_slice(&pubkey);
    let keypair = KeyPair::new(nacl_secret, pubkey);

    let wallet = Wallet::<V4R2>::derive_default(keypair)
        .map_err(|e| GradienceError::Validation(format!("ton wallet derive failed: {}", e)))?;

    let dest_address = to.parse()
        .map_err(|e: <tlb_ton::MsgAddress as std::str::FromStr>::Err| GradienceError::Validation(format!("invalid ton address: {}", e)))?;

    let action = SendMsgAction {
        mode: 3, // pay fees separately + carry remaining value
        message: Message::<()>::transfer(dest_address, BigUint::from(amount_nanoton), false)
            .normalize()
            .map_err(|e| GradienceError::Validation(format!("ton message normalize failed: {}", e)))?,
    };

    let expire_at = chrono::DateTime::UNIX_EPOCH;
    let msg = wallet.create_external_message(expire_at, seqno, [action], true)
        .map_err(|e| GradienceError::Validation(format!("ton external message failed: {}", e)))?;

    let cell = msg.to_cell(())
        .map_err(|e| GradienceError::Validation(format!("ton cell build failed: {}", e)))?;
    let boc = BoC::from_root(cell);

    let bytes = boc.serialize(BagOfCellsArgs { has_idx: false, has_crc32c: true })
        .map_err(|e| GradienceError::Validation(format!("ton boc serialize failed: {}", e)))?;
    Ok(bytes)
}

/// Jetton transfer message body.
/// TL-B: transfer#0f8a7ea5 query_id:uint64 amount:(VarUInteger 16)
///   destination:MsgAddress response_destination:MsgAddress
///   custom_payload:(Maybe ^Cell) forward_ton_amount:(VarUInteger 16)
///   forward_payload:(Either Cell ^Cell) = InternalMsgBody;
#[derive(Debug, Clone)]
pub struct JettonTransferBody {
    pub query_id: u64,
    pub amount: BigUint,
    pub destination: MsgAddress,
    pub response_destination: MsgAddress,
    pub custom_payload: Option<Arc<tlb_ton::Cell>>,
    pub forward_ton_amount: BigUint,
    pub forward_payload: Option<Arc<tlb_ton::Cell>>,
}

impl CellSerialize for JettonTransferBody {
    type Args = ();

    fn store(&self, builder: &mut CellBuilder, _: Self::Args) -> std::result::Result<(), tlb_ton::StringError> {
        use tlb_ton::bits::ser::BitWriterExt;
        builder
            .pack_as::<_, NBits<32>>(0x0f8a7ea5u32, ())?
            .pack(self.query_id, ())?
            .pack_as::<_, &Grams>(&self.amount, ())?
            .pack(self.destination, ())?
            .pack(self.response_destination, ())?
            .store_as::<_, Option<Ref>>(self.custom_payload.clone(), ())?
            .pack_as::<_, &Grams>(&self.forward_ton_amount, ())?
            .store_as::<_, EitherInlineOrRef>(self.forward_payload.clone(), ())?;
        Ok(())
    }
}

/// Build a TON V4R2 external message that sends a Jetton transfer.
///
/// `amount_jetton` is in the Jetton's base units (e.g. 0.1 USDT = 100000 for 6 decimals).
/// `ton_amount_nanoton` is the TON value attached to the message (gas, ~0.05 TON).
pub fn build_jetton_transfer_tx(
    seed: &[u8],
    jetton_wallet: &str,
    recipient: &str,
    amount_jetton: u128,
    ton_amount_nanoton: u64,
    seqno: u32,
) -> Result<Vec<u8>> {
    let secret = ton_secret_from_seed(seed);
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret);
    let pubkey = signing_key.verifying_key().to_bytes();

    let mut nacl_secret = [0u8; 64];
    nacl_secret[..32].copy_from_slice(&secret);
    nacl_secret[32..].copy_from_slice(&pubkey);
    let keypair = KeyPair::new(nacl_secret, pubkey);

    let wallet = Wallet::<V4R2>::derive_default(keypair)
        .map_err(|e| GradienceError::Validation(format!("ton wallet derive failed: {}", e)))?;

    let jetton_wallet_addr: MsgAddress = jetton_wallet
        .parse()
        .map_err(|e: <MsgAddress as std::str::FromStr>::Err| {
            GradienceError::Validation(format!("invalid jetton wallet address: {}", e))
        })?;
    let recipient_addr: MsgAddress = recipient
        .parse()
        .map_err(|e: <MsgAddress as std::str::FromStr>::Err| {
            GradienceError::Validation(format!("invalid recipient ton address: {}", e))
        })?;

    let sender_addr: MsgAddress = ton_address_from_seed(seed)
        .map_err(|e| GradienceError::Validation(format!("ton address derive failed: {}", e)))?
        .parse()
        .map_err(|e: <MsgAddress as std::str::FromStr>::Err| {
            GradienceError::Validation(format!("ton address parse failed: {}", e))
        })?;

    let body = JettonTransferBody {
        query_id: rand::random(),
        amount: BigUint::from(amount_jetton),
        destination: recipient_addr,
        response_destination: sender_addr,
        custom_payload: None,
        forward_ton_amount: BigUint::from(0u64),
        forward_payload: None,
    };

    let body_cell = body
        .to_cell(())
        .map_err(|e| GradienceError::Validation(format!("jetton body cell build failed: {}", e)))?;

    let msg = tlb_ton::message::Message::<tlb_ton::Cell> {
        info: tlb_ton::message::CommonMsgInfo::transfer(
            jetton_wallet_addr,
            BigUint::from(ton_amount_nanoton),
            false,
        ),
        init: None,
        body: body_cell,
    };

    let action = SendMsgAction {
        mode: 3,
        message: msg
            .normalize()
            .map_err(|e| GradienceError::Validation(format!("ton message normalize failed: {}", e)))?,
    };

    let expire_at = chrono::DateTime::UNIX_EPOCH;
    let msg = wallet
        .create_external_message(expire_at, seqno, [action], true)
        .map_err(|e| GradienceError::Validation(format!("ton external message failed: {}", e)))?;

    let cell = msg
        .to_cell(())
        .map_err(|e| GradienceError::Validation(format!("ton cell build failed: {}", e)))?;
    let boc = BoC::from_root(cell);

    let bytes = boc
        .serialize(BagOfCellsArgs {
            has_idx: false,
            has_crc32c: true,
        })
        .map_err(|e| GradienceError::Validation(format!("ton boc serialize failed: {}", e)))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_ata_known_vector() {
        // owner = 8uAPC2UxiBjKmUksVVwUA6q4RctiXkgSAsovBR39cd1i
        // mint  = EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v (USDC)
        let ata = find_associated_token_address(
            "8uAPC2UxiBjKmUksVVwUA6q4RctiXkgSAsovBR39cd1i",
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        ).unwrap();
        // Length must be a valid base58-encoded 32-byte pubkey.
        let decoded = bs58::decode(&ata).into_vec().unwrap();
        assert_eq!(decoded.len(), 32);
    }

    #[test]
    fn test_ton_address_derivation() {
        // Known test vector from ton-contracts docs:
        // Mnemonic: "jewel loop vast intact snack drip fatigue lunch erode green indoor balance together scrub hen monster hour narrow banner warfare increase panel sound spell"
        // Expected address: UQA7RMTgzvcyxNNLmK2HdklOvFE8_KNMa-btKZ0dPU1UsqfC
        // We won't replicate mnemonic here, but we can at least assert that a deterministic seed yields a valid TON address format.
        let seed = [0u8; 32];
        let addr = ton_address_from_seed(&seed).unwrap();
        assert!(addr.starts_with("EQ") || addr.starts_with("UQ"));
    }

    #[test]
    fn test_jetton_transfer_body_cell_encoding() {
        let addr: MsgAddress = "EQBGXZ9ddZeWypx8EkJieHJX75ct0bpkmu0Y4YoYr3NM0Z9e".parse().unwrap();
        let body = JettonTransferBody {
            query_id: 123,
            amount: BigUint::from(1000000u64),
            destination: addr,
            response_destination: addr,
            custom_payload: None,
            forward_ton_amount: BigUint::from(0u64),
            forward_payload: None,
        };
        let cell = body.to_cell(()).unwrap();
        // Verify OP code 0x0f8a7ea5 is present in first 32 bits
        let first_4_bytes: [u8; 4] = cell.data.as_raw_slice()[0..4].try_into().unwrap();
        assert_eq!(first_4_bytes, [0x0f, 0x8a, 0x7e, 0xa5]);
        // No refs for empty custom_payload / forward_payload
        assert!(cell.references.is_empty());
    }

    #[test]
    fn test_build_jetton_transfer_tx_produces_boc() {
        let seed = [0u8; 32];
        let jetton_wallet = "EQBGXZ9ddZeWypx8EkJieHJX75ct0bpkmu0Y4YoYr3NM0Z9e";
        // Derive a valid recipient from a different seed
        let mut recipient_seed = [1u8; 32];
        recipient_seed[0] = 1;
        let recipient = ton_address_from_seed(&recipient_seed).unwrap();
        let result = build_jetton_transfer_tx(
            &seed, jetton_wallet, &recipient, 1000, 50_000_000, 1
        );
        match result {
            Ok(tx_boc) => {
                assert!(!tx_boc.is_empty());
                assert_eq!(&tx_boc[0..4], [0xb5, 0xee, 0x9c, 0x72]);
            }
            Err(e) => panic!("build_jetton_transfer_tx failed: {:?}", e),
        }
    }
}
