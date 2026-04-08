use crate::audit::merkle::MerkleTree;
use crate::ows::signing::{eth_address_from_secret_key, sign_eth_transaction};
use crate::rpc::evm::EvmRpcClient;
use anyhow::Result;
use sha3::Digest;
use sqlx::{Pool, Sqlite};

fn encode_u256(val: u64) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[24..].copy_from_slice(&val.to_be_bytes());
    buf
}

fn build_anchor_calldata(
    root: [u8; 32],
    prev_root: [u8; 32],
    log_start: u64,
    log_end: u64,
    leaf_count: u64,
) -> Vec<u8> {
    let selector: [u8; 4] = [0xc7, 0x4d, 0x7d, 0x6d];
    let mut data = Vec::with_capacity(4 + 5 * 32);
    data.extend_from_slice(&selector);
    data.extend_from_slice(&root);
    data.extend_from_slice(&prev_root);
    data.extend_from_slice(&encode_u256(log_start));
    data.extend_from_slice(&encode_u256(log_end));
    data.extend_from_slice(&encode_u256(leaf_count));
    data
}

fn compute_audit_leaf(log: &gradience_db::models::AuditLog) -> [u8; 32] {
    let mut hasher = sha3::Keccak256::new();
    hasher.update(&log.id.to_be_bytes());
    hasher.update(log.wallet_id.as_bytes());
    hasher.update(log.action.as_bytes());
    hasher.update(log.decision.as_bytes());
    if let Some(ref tx_hash) = log.tx_hash {
        hasher.update(tx_hash.as_bytes());
    }
    hasher.update(log.created_at.timestamp().to_be_bytes());
    hasher.finalize().as_slice().try_into().unwrap()
}

pub struct AnchorService {
    rpc_url: String,
    contract_address: String,
    private_key: [u8; 32],
    chain_id: u64,
}

impl AnchorService {
    pub fn from_env() -> Result<Option<Self>> {
        let Ok(rpc_url) = std::env::var("ANCHOR_RPC_URL") else {
            return Ok(None);
        };
        let contract_address = std::env::var("ANCHOR_CONTRACT_ADDRESS")
            .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".into());
        let private_key_hex = std::env::var("ANCHOR_PRIVATE_KEY")
            .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000000000000000000000000000".into());
        let private_key = hex::decode(private_key_hex.trim_start_matches("0x"))?
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid private key length"))?;
        let chain_id = std::env::var("ANCHOR_CHAIN_ID")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8453);
        Ok(Some(Self {
            rpc_url,
            contract_address,
            private_key,
            chain_id,
        }))
    }

    pub async fn anchor_unanchored_logs(
        &self,
        db: &Pool<Sqlite>,
        wallet_id: &str,
        batch_size: i64,
    ) -> Result<Option<String>> {
        let mut logs = gradience_db::queries::list_unanchored_audit_logs_for_wallet(db, wallet_id, batch_size).await?;
        if logs.is_empty() {
            return Ok(None);
        }
        logs.sort_by_key(|l| l.id);

        let leaves: Vec<[u8; 32]> = logs.iter().map(compute_audit_leaf).collect();
        let tree = MerkleTree::new(leaves);
        let root = tree.root;

        let prev_root = gradience_db::queries::get_latest_anchor_batch(db)
            .await?
            .map(|b| {
                let mut r = [0u8; 32];
                hex::decode_to_slice(&b.root, &mut r).ok();
                r
            })
            .unwrap_or([0u8; 32]);

        let log_start = logs.first().unwrap().id as u64;
        let log_end = logs.last().unwrap().id as u64;
        let leaf_count = logs.len() as u64;

        let data = build_anchor_calldata(root, prev_root, log_start, log_end, leaf_count);

        let to = self.contract_address.trim_start_matches("0x");
        let from_addr = eth_address_from_secret_key(&self.private_key)?;
        let client = EvmRpcClient::new("evm", &self.rpc_url)?;
        let nonce = client.get_transaction_count(&from_addr).await?;
        let gas_price_hex = client.get_gas_price().await?;
        let gas_price = u128::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16)?;

        let signed_tx = sign_eth_transaction(
            &self.private_key,
            nonce,
            gas_price,
            150000,
            to,
            0,
            &data,
            self.chain_id,
        )?;

        let tx_hash = client
            .send_raw_transaction(&format!("0x{}", hex::encode(&signed_tx)))
            .await?;

        let root_hex = format!("0x{}", hex::encode(root));
        let prev_root_hex = if prev_root == [0u8; 32] {
            None
        } else {
            Some(format!("0x{}", hex::encode(prev_root)))
        };

        gradience_db::queries::insert_anchor_batch(
            db,
            &root_hex,
            prev_root_hex.as_deref(),
            log_start as i64,
            log_end as i64,
            leaf_count as i32,
            &tx_hash,
            None,
        )
        .await?;

        let ids: Vec<i64> = logs.iter().map(|l| l.id).collect();
        gradience_db::queries::mark_logs_anchored(
            db,
            &ids,
            &format!("0x{}", hex::encode(root)),
            &tx_hash,
        )
        .await?;

        Ok(Some(tx_hash))
    }
}
