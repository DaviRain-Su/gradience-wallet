use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MppPaymentRequest {
    pub sender_wallet_id: String,
    pub sender_address: Option<String>,
    pub recipients: Vec<MppRecipient>,
    pub token_address: String,
    pub chain: String,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MppRecipient {
    pub address: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MppPaymentResult {
    pub batch_tx_hash: String,
    pub recipient_count: usize,
}

/// Real on-chain payload produced by `build_batch`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchTransferPayload {
    Evm {
        /// Contract / recipient address
        to: String,
        /// Total native value (hex)
        value: String,
        /// ABI-encoded calldata (hex)
        data: String,
    },
    Solana {
        /// Unsigned legacy transaction, base64-encoded
        serialized_tx: String,
    },
}

/// Multicall3 contract address, identical across all EVM chains.
pub const MULTICALL3_ADDRESS: &str = "0xcA11bde05977b3631167028862bE2a173976CA11";

pub struct MppService;

impl Default for MppService {
    fn default() -> Self {
        Self::new()
    }
}

impl MppService {
    pub fn new() -> Self {
        Self
    }

    /// Build a real batch-transfer payload suitable for direct on-chain submission.
    ///
    /// - **EVM**: packs ERC20 / native transfers into a single Multicall3 `aggregate` call.
    /// - **Solana**: builds a single **unsigned** legacy transaction containing multiple
    ///   SPL Token transfer instructions.
    pub fn build_batch(
        &self,
        req: &MppPaymentRequest,
    ) -> Result<BatchTransferPayload, crate::error::GradienceError> {
        if req.recipients.is_empty() {
            return Err(crate::error::GradienceError::InvalidCredential(
                "no recipients".into(),
            ));
        }

        if req.chain.starts_with("solana") {
            self.build_batch_solana(req)
        } else {
            self.build_batch_evm(req)
        }
    }

    fn build_batch_evm(
        &self,
        req: &MppPaymentRequest,
    ) -> Result<BatchTransferPayload, crate::error::GradienceError> {
        let is_native = req.token_address.is_empty()
            || req.token_address == "0x0000000000000000000000000000000000000000";

        let mut total_native_value = alloy::primitives::U256::ZERO;
        let mut calls: Vec<Call3Value> = Vec::with_capacity(req.recipients.len());

        for r in &req.recipients {
            let amount_u128 = r.amount.parse::<u128>().map_err(|e| {
                crate::error::GradienceError::Validation(format!("bad amount: {}", e))
            })?;

            if is_native {
                total_native_value += alloy::primitives::U256::from(amount_u128);
            }

            let target: alloy::primitives::Address = if is_native {
                r.address.parse().map_err(|e| {
                    crate::error::GradienceError::Validation(format!("bad address: {}", e))
                })?
            } else {
                req.token_address.parse().map_err(|e| {
                    crate::error::GradienceError::Validation(format!("bad token address: {}", e))
                })?
            };

            let call_data = if is_native {
                alloy::primitives::Bytes::new()
            } else {
                let to_addr: alloy::primitives::Address = r.address.parse().map_err(|e| {
                    crate::error::GradienceError::Validation(format!("bad recipient: {}", e))
                })?;

                let mut calldata = vec![0xa9u8, 0x05, 0x9c, 0xbb];
                calldata.extend_from_slice(&[0u8; 12]);
                calldata.extend_from_slice(to_addr.as_slice());
                let mut amount_bytes = [0u8; 32];
                amount_bytes[16..].copy_from_slice(&amount_u128.to_be_bytes());
                calldata.extend_from_slice(&amount_bytes);

                alloy::primitives::Bytes::from(calldata)
            };

            calls.push(Call3Value {
                target,
                allow_failure: false,
                value: if is_native {
                    alloy::primitives::U256::from(amount_u128)
                } else {
                    alloy::primitives::U256::ZERO
                },
                call_data,
            });
        }

        let data = encode_multicall3_aggregate(&calls)?;

        Ok(BatchTransferPayload::Evm {
            to: MULTICALL3_ADDRESS.into(),
            value: format!("0x{:x}", total_native_value),
            data: format!("0x{}", hex::encode(&data)),
        })
    }

    fn build_batch_solana(
        &self,
        req: &MppPaymentRequest,
    ) -> Result<BatchTransferPayload, crate::error::GradienceError> {
        let sender = req.sender_address.as_deref().ok_or_else(|| {
            crate::error::GradienceError::Validation(
                "sender_address required for Solana batch".into(),
            )
        })?;

        let is_native = req.token_address.is_empty()
            || req.token_address == "So11111111111111111111111111111111111111112";
        let mint = if is_native {
            None
        } else {
            Some(req.token_address.as_str())
        };

        let mut all_keys: Vec<[u8; 32]> = Vec::new();
        let mut instructions: Vec<(u8, Vec<(u8, bool, bool)>, Vec<u8>)> = Vec::new();

        let mut key_index = |key: &[u8; 32]| -> u8 {
            if let Some(pos) = all_keys.iter().position(|k| k == key) {
                pos as u8
            } else {
                all_keys.push(*key);
                (all_keys.len() - 1) as u8
            }
        };

        let sender_pk = decode_solana_pubkey(sender)?;
        let sender_idx = key_index(&sender_pk);

        let system_program = decode_solana_pubkey("11111111111111111111111111111111")?;
        let system_idx = key_index(&system_program);

        for r in &req.recipients {
            let amount = r.amount.parse::<u64>().map_err(|e| {
                crate::error::GradienceError::Validation(format!("bad amount: {}", e))
            })?;
            let recipient_pk = decode_solana_pubkey(&r.address)?;
            let recipient_idx = key_index(&recipient_pk);

            if is_native {
                let accounts = vec![(sender_idx, true, true), (recipient_idx, false, true)];
                let mut data = vec![0u8; 12];
                data[0..4].copy_from_slice(&2u32.to_le_bytes());
                data[4..12].copy_from_slice(&amount.to_le_bytes());
                instructions.push((system_idx, accounts, data));
            } else {
                let mint_pk = decode_solana_pubkey(mint.unwrap())?;
                let _mint_idx = key_index(&mint_pk);

                let token_program =
                    decode_solana_pubkey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")?;
                let token_idx = key_index(&token_program);

                let sender_ata_b58 =
                    crate::ows::signing::find_associated_token_address(sender, mint.unwrap())
                        .map_err(|e| crate::error::GradienceError::Validation(e.to_string()))?;
                let recipient_ata_b58 =
                    crate::ows::signing::find_associated_token_address(&r.address, mint.unwrap())
                        .map_err(|e| crate::error::GradienceError::Validation(e.to_string()))?;
                let sender_ata = decode_solana_pubkey(&sender_ata_b58)?;
                let recipient_ata = decode_solana_pubkey(&recipient_ata_b58)?;
                let sender_ata_idx = key_index(&sender_ata);
                let recipient_ata_idx = key_index(&recipient_ata);

                let accounts = vec![
                    (sender_ata_idx, false, true),
                    (recipient_ata_idx, false, true),
                    (sender_idx, true, false),
                ];
                let mut data = vec![0u8; 9];
                data[0] = 3u8;
                data[1..9].copy_from_slice(&amount.to_le_bytes());
                instructions.push((token_idx, accounts, data));
            }
        }

        let blockhash = [0u8; 32];

        let message =
            build_solana_legacy_message(&sender_pk, &all_keys, &blockhash, &instructions)?;

        let mut tx = encode_compact_u16(1);
        tx.extend_from_slice(&[0u8; 64]);
        tx.extend_from_slice(&message);

        Ok(BatchTransferPayload::Solana {
            serialized_tx: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &tx),
        })
    }
}

struct Call3Value {
    target: alloy::primitives::Address,
    allow_failure: bool,
    value: alloy::primitives::U256,
    call_data: alloy::primitives::Bytes,
}

fn encode_multicall3_aggregate(
    calls: &[Call3Value],
) -> Result<Vec<u8>, crate::error::GradienceError> {
    use alloy::primitives::U256;

    // selector for aggregate((address,bool,uint256,bytes)[])
    let selector: [u8; 4] = [0x17, 0x49, 0xe1, 0xe3];

    let mut head = Vec::new();
    let mut tail = Vec::new();

    // offset to dynamic array arg
    head.extend_from_slice(&U256::from(32).to_be_bytes::<32>());
    // array length
    head.extend_from_slice(&U256::from(calls.len()).to_be_bytes::<32>());

    for call in calls {
        // offset to this struct's dynamic tail (relative to start of array)
        let struct_offset = 32 + calls.len() * 32 + tail.len();
        head.extend_from_slice(&U256::from(struct_offset).to_be_bytes::<32>());

        // target (address padded to 32)
        let mut addr = [0u8; 32];
        addr[12..].copy_from_slice(call.target.as_slice());
        tail.extend_from_slice(&addr);
        // allowFailure (bool)
        tail.extend_from_slice(
            &U256::from(if call.allow_failure { 1u64 } else { 0u64 }).to_be_bytes::<32>(),
        );
        // value (uint256)
        tail.extend_from_slice(&call.value.to_be_bytes::<32>());
        // offset to callData inside struct (always 4 * 32)
        tail.extend_from_slice(&U256::from(4 * 32).to_be_bytes::<32>());
        // callData length
        tail.extend_from_slice(&U256::from(call.call_data.len()).to_be_bytes::<32>());
        // callData content
        tail.extend_from_slice(&call.call_data);
        // pad to 32 bytes
        let padding = (32 - call.call_data.len() % 32) % 32;
        tail.extend_from_slice(&vec![0u8; padding]);
    }

    let mut result = selector.to_vec();
    result.extend_from_slice(&head);
    result.extend_from_slice(&tail);
    Ok(result)
}

fn decode_solana_pubkey(b58: &str) -> Result<[u8; 32], crate::error::GradienceError> {
    let bytes = bs58::decode(b58)
        .into_vec()
        .map_err(|e| crate::error::GradienceError::Validation(format!("invalid base58: {}", e)))?;
    if bytes.len() != 32 {
        return Err(crate::error::GradienceError::Validation(
            "pubkey must be 32 bytes".into(),
        ));
    }
    Ok(bytes.try_into().unwrap())
}

fn encode_compact_u16(value: u16) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut v = value;
    loop {
        let mut b = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            b |= 0x80;
        }
        buf.push(b);
        if v == 0 {
            break;
        }
    }
    buf
}

fn build_solana_legacy_message(
    _payer: &[u8; 32],
    account_keys: &[[u8; 32]],
    recent_blockhash: &[u8; 32],
    instructions: &[(u8, Vec<(u8, bool, bool)>, Vec<u8>)],
) -> Result<Vec<u8>, crate::error::GradienceError> {
    let num_required_signatures: u8 = 1;
    let num_readonly_signed_accounts: u8 = 0;

    let readonly_unsigned: u8 = account_keys
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 0)
        .filter(|(i, _)| {
            let idx = *i as u8;
            !instructions
                .iter()
                .any(|(_, accts, _)| accts.iter().any(|(a, _, w)| *a == idx && *w))
        })
        .count() as u8;

    let mut message = vec![];
    message.push(num_required_signatures);
    message.push(num_readonly_signed_accounts);
    message.push(readonly_unsigned);
    message.extend_from_slice(&encode_compact_u16(account_keys.len() as u16));
    for key in account_keys {
        message.extend_from_slice(key);
    }
    message.extend_from_slice(recent_blockhash);
    message.extend_from_slice(&encode_compact_u16(instructions.len() as u16));

    for (prog_idx, accounts, data) in instructions {
        message.push(*prog_idx);
        message.extend_from_slice(&encode_compact_u16(accounts.len() as u16));
        for (idx, is_signer, is_writable) in accounts {
            let mut meta = 0u8;
            if *is_signer {
                meta |= 0x80;
            }
            if *is_writable {
                meta |= 0x40;
            }
            message.push(meta | idx);
        }
        message.extend_from_slice(&encode_compact_u16(data.len() as u16));
        message.extend_from_slice(data);
    }

    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_batch_evm_erc20() {
        let svc = MppService::new();
        let req = MppPaymentRequest {
            sender_wallet_id: "wallet-1".into(),
            sender_address: None,
            recipients: vec![MppRecipient {
                address: "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C".into(),
                amount: "1000000".into(),
            }],
            token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(),
            chain: "base".into(),
            memo: None,
        };
        let batch = svc.build_batch(&req).unwrap();
        match batch {
            BatchTransferPayload::Evm { to, value, data } => {
                assert_eq!(to, MULTICALL3_ADDRESS);
                assert_eq!(value, "0x0");
                assert!(data.starts_with("0x1749e1e3"));
            }
            _ => panic!("expected EVM payload"),
        }
    }

    #[test]
    fn test_build_batch_evm_native() {
        let svc = MppService::new();
        let req = MppPaymentRequest {
            sender_wallet_id: "wallet-1".into(),
            sender_address: None,
            recipients: vec![
                MppRecipient {
                    address: "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C".into(),
                    amount: "1000000".into(),
                },
                MppRecipient {
                    address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".into(),
                    amount: "2000000".into(),
                },
            ],
            token_address: "0x0000000000000000000000000000000000000000".into(),
            chain: "base".into(),
            memo: None,
        };
        let batch = svc.build_batch(&req).unwrap();
        match batch {
            BatchTransferPayload::Evm { to, value, data } => {
                assert_eq!(to, MULTICALL3_ADDRESS);
                assert_eq!(value, "0x2dc6c0"); // 1000000 + 2000000 = 3000000
                assert!(data.starts_with("0x1749e1e3"));
            }
            _ => panic!("expected EVM payload"),
        }
    }

    #[test]
    fn test_build_batch_solana_spl() {
        let svc = MppService::new();
        let req = MppPaymentRequest {
            sender_wallet_id: "wallet-1".into(),
            sender_address: Some("4fP7AaKvKk4ePH8ag46G3cvLCG3A5NzL65FcvHEZgZsd".into()),
            recipients: vec![MppRecipient {
                address: "6s5wd3EWpPdPsPMvoWzBQz9qWiC9kbWLUqQMrYMfVJde".into(),
                amount: "1000000".into(),
            }],
            token_address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into(),
            chain: "solana".into(),
            memo: None,
        };
        let batch = svc.build_batch(&req).unwrap();
        match batch {
            BatchTransferPayload::Solana { serialized_tx } => {
                assert!(!serialized_tx.is_empty());
                let decoded = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    &serialized_tx,
                )
                .unwrap();
                assert!(decoded.len() > 64 + 32); // sig + blockhash
            }
            _ => panic!("expected Solana payload"),
        }
    }

    #[test]
    fn test_build_batch_empty_recipients_fails() {
        let svc = MppService::new();
        let req = MppPaymentRequest {
            sender_wallet_id: "wallet-1".into(),
            sender_address: None,
            recipients: vec![],
            token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(),
            chain: "base".into(),
            memo: None,
        };
        assert!(svc.build_batch(&req).is_err());
    }
}
