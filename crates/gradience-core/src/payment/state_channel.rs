use crate::error::{GradienceError, Result};
use secp256k1::SecretKey;
use sha3::{Digest, Keccak256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Ethereum personal-sign message hash.
/// `hash = keccak256("\x19Ethereum Signed Message:\n32" || digest)`
pub fn eth_personal_hash(digest: &[u8; 32]) -> [u8; 32] {
    let prefix = b"\x19Ethereum Signed Message:\n32";
    let mut buf = Vec::with_capacity(prefix.len() + 32);
    buf.extend_from_slice(prefix);
    buf.extend_from_slice(digest);
    keccak256(&buf)
}

/// Build the state-channel digest that the Solidity contract expects:
/// `keccak256(abi.encodePacked(channelId, nonce, amount))`
pub fn state_channel_digest(channel_id: &[u8; 32], nonce: u64, amount: u128) -> [u8; 32] {
    let mut buf = Vec::with_capacity(32 + 32 + 32);
    buf.extend_from_slice(channel_id);
    // Pad nonce and amount to 32 bytes to match Solidity uint256 abi.encodePacked
    let mut nonce_bytes = [0u8; 32];
    nonce_bytes[24..].copy_from_slice(&nonce.to_be_bytes());
    buf.extend_from_slice(&nonce_bytes);
    let mut amount_bytes = [0u8; 32];
    amount_bytes[16..].copy_from_slice(&amount.to_be_bytes());
    buf.extend_from_slice(&amount_bytes);
    keccak256(&buf)
}

/// Sign a state update using an Ethereum personal-sign style signature.
/// Returns a 65-byte recoverable signature `[r (32) || s (32) || v (1)]`.
pub fn sign_state_update(
    secret: &[u8; 32],
    channel_id: &[u8; 32],
    nonce: u64,
    amount: u128,
) -> Result<[u8; 65]> {
    let sk = SecretKey::from_slice(secret).map_err(|e| GradienceError::Signature(e.to_string()))?;
    let secp = secp256k1::Secp256k1::new();

    let digest = state_channel_digest(channel_id, nonce, amount);
    let eth_hash = eth_personal_hash(&digest);
    let msg = secp256k1::Message::from_digest_slice(&eth_hash)
        .map_err(|e| GradienceError::Signature(e.to_string()))?;

    let sig = secp.sign_ecdsa_recoverable(&msg, &sk);
    let (rec_id, compact) = sig.serialize_compact();
    let mut out = [0u8; 65];
    out[0..32].copy_from_slice(&compact[0..32]);
    out[32..64].copy_from_slice(&compact[32..64]);
    out[64] = rec_id.to_i32() as u8 + 27;
    Ok(out)
}

/// Recover the Ethereum address from a 65-byte signature.
pub fn recover_signer(eth_hash: &[u8; 32], sig: &[u8; 65]) -> Result<String> {
    let secp = secp256k1::Secp256k1::new();
    let msg = secp256k1::Message::from_digest_slice(eth_hash)
        .map_err(|e| GradienceError::Signature(e.to_string()))?;

    let mut rec_id = sig[64];
    if rec_id >= 27 {
        rec_id -= 27;
    }
    let rid = secp256k1::ecdsa::RecoveryId::from_i32(rec_id as i32)
        .map_err(|e| GradienceError::Signature(e.to_string()))?;
    let compact = &sig[0..64];
    let recoverable = secp256k1::ecdsa::RecoverableSignature::from_compact(compact, rid)
        .map_err(|e| GradienceError::Signature(e.to_string()))?;
    let pk = secp
        .recover_ecdsa(&msg, &recoverable)
        .map_err(|e| GradienceError::Signature(e.to_string()))?;

    let uncompressed = pk.serialize_uncompressed();
    let hash = keccak256(&uncompressed[1..]);
    let addr = format!("0x{}", hex::encode(&hash[12..]));
    Ok(addr)
}

/// Verify that `signature` over `(channel_id, nonce, amount)` was signed by `expected_address`.
pub fn verify_state_update(
    channel_id: &[u8; 32],
    nonce: u64,
    amount: u128,
    signature: &[u8; 65],
    expected_address: &str,
) -> Result<bool> {
    let digest = state_channel_digest(channel_id, nonce, amount);
    let eth_hash = eth_personal_hash(&digest);
    let signer = recover_signer(&eth_hash, signature)?;
    Ok(signer.eq_ignore_ascii_case(expected_address))
}

/// Off-chain state update.
#[derive(Debug, Clone)]
pub struct StateUpdate {
    pub channel_id: [u8; 32],
    pub nonce: u64,
    pub amount: u128,
    pub signature: [u8; 65],
}

/// In-memory payer-side channel state.
#[derive(Debug, Clone)]
pub struct LocalChannel {
    pub channel_id: [u8; 32],
    pub payer_address: String,
    pub payee_address: String,
    pub deposit: u128,
    pub challenge_period_secs: u64,
    pub expires_at: u64,
    pub next_nonce: u64,
    pub cumulative_spent: u128,
    pub secret: [u8; 32],
}

/// Simple payer-side manager for state channels.
#[derive(Debug, Clone)]
pub struct StateChannelManager {
    channels: Arc<Mutex<HashMap<[u8; 32], LocalChannel>>>,
}

impl Default for StateChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StateChannelManager {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a channel that the payer already opened on-chain.
    pub fn register_channel(
        &self,
        channel_id: [u8; 32],
        payer_address: String,
        payee_address: String,
        deposit: u128,
        challenge_period_secs: u64,
        expires_at: u64,
        secret: [u8; 32],
    ) {
        let ch = LocalChannel {
            channel_id,
            payer_address,
            payee_address,
            deposit,
            challenge_period_secs,
            expires_at,
            next_nonce: 1,
            cumulative_spent: 0,
            secret,
        };
        self.channels.lock().unwrap().insert(channel_id, ch);
    }

    /// Generate the next signed state update for a given cost increment.
    pub fn next_update(&self, channel_id: &[u8; 32], cost: u128) -> Result<StateUpdate> {
        let mut map = self.channels.lock().unwrap();
        let ch = map
            .get_mut(channel_id)
            .ok_or_else(|| GradienceError::Validation("Unknown channel".into()))?;

        let cumulative = ch.cumulative_spent.saturating_add(cost);
        if cumulative > ch.deposit {
            return Err(GradienceError::Validation(
                "State channel deposit exceeded".into(),
            ));
        }

        let nonce = ch.next_nonce;
        ch.next_nonce += 1;
        ch.cumulative_spent = cumulative;

        let signature = sign_state_update(&ch.secret, channel_id, nonce, cumulative)?;
        Ok(StateUpdate {
            channel_id: *channel_id,
            nonce,
            amount: cumulative,
            signature,
        })
    }

    pub fn channel(&self, channel_id: &[u8; 32]) -> Option<LocalChannel> {
        self.channels.lock().unwrap().get(channel_id).cloned()
    }

    pub fn remove_channel(&self, channel_id: &[u8; 32]) {
        self.channels.lock().unwrap().remove(channel_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify_state_update() {
        let secret = [1u8; 32];
        let channel_id = [2u8; 32];
        let nonce = 1u64;
        let amount = 100u128;

        let sig = sign_state_update(&secret, &channel_id, nonce, amount).unwrap();
        let addr = eth_address_from_secret_key(&secret).unwrap();

        assert!(verify_state_update(&channel_id, nonce, amount, &sig, &addr).unwrap());
        assert!(!verify_state_update(
            &channel_id,
            nonce,
            amount,
            &sig,
            "0x0000000000000000000000000000000000000000"
        )
        .unwrap());
    }

    #[test]
    fn test_manager_next_update() {
        let secret = [7u8; 32];
        let channel_id = [8u8; 32];
        let addr = eth_address_from_secret_key(&secret).unwrap();

        let mgr = StateChannelManager::new();
        mgr.register_channel(
            channel_id,
            addr.clone(),
            "0xPayee".into(),
            500,
            300,
            u64::MAX,
            secret,
        );

        let up1 = mgr.next_update(&channel_id, 100).unwrap();
        assert_eq!(up1.nonce, 1);
        assert_eq!(up1.amount, 100);
        assert!(
            verify_state_update(&channel_id, up1.nonce, up1.amount, &up1.signature, &addr).unwrap()
        );

        let up2 = mgr.next_update(&channel_id, 150).unwrap();
        assert_eq!(up2.nonce, 2);
        assert_eq!(up2.amount, 250); // 100 + 150
        assert!(
            verify_state_update(&channel_id, up2.nonce, up2.amount, &up2.signature, &addr).unwrap()
        );

        // 250 + 300 = 550 > 500 deposit -> should fail
        let up3 = mgr.next_update(&channel_id, 300);
        assert!(up3.is_err());
    }

    fn eth_address_from_secret_key(secret: &[u8; 32]) -> Result<String> {
        crate::ows::signing::eth_address_from_secret_key(secret)
    }

    #[test]
    fn debug_signature_for_xlayer() {
        let secret =
            hex::decode("bebff393a40d6aabe1e7fd66bd7299f094255ed574b4abc08f5329b9629ee4c9")
                .unwrap()
                .try_into()
                .unwrap();
        let channel_id_hex = "aeceee7c3af302924c0ad1096bce357048cecbce73bfddaa4b7dba7d7bb66db9";
        let mut channel_id = [0u8; 32];
        hex::decode_to_slice(channel_id_hex, &mut channel_id).unwrap();

        let sig = sign_state_update(&secret, &channel_id, 1, 500000000000000).unwrap();
        let addr = eth_address_from_secret_key(&secret).unwrap();
        println!("payer_address: {}", addr);
        println!("signature: 0x{}", hex::encode(sig));
        assert!(verify_state_update(&channel_id, 1, 500000000000000, &sig, &addr).unwrap());
    }
}
