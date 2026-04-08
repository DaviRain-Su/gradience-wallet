use serde::{Deserialize, Serialize};
use crate::error::{GradienceError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X402Requirement {
    pub scheme: String,
    pub network: String,
    pub token_address: String,
    pub recipient: String,
    pub amount: String,
    pub deadline: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X402Payment {
    pub requirement: X402Requirement,
    pub signature: String,
    pub tx_hash: Option<String>,
}

pub struct X402Service;

impl X402Service {
    pub fn new() -> Self {
        Self
    }

    pub fn create_requirement(
        &self,
        recipient: &str,
        amount: &str,
        token_address: &str,
        deadline: u64,
        network: Option<&str>,
    ) -> Result<X402Requirement> {
        if recipient.is_empty() {
            return Err(GradienceError::InvalidCredential("invalid recipient address".into()));
        }
        let net = network.unwrap_or("base");
        if net.starts_with("stellar") {
            // Stellar addresses are not hex
            return Ok(X402Requirement {
                scheme: "exact".into(),
                network: net.into(),
                token_address: token_address.into(),
                recipient: recipient.into(),
                amount: amount.into(),
                deadline,
            });
        }
        if !recipient.starts_with("0x") {
            return Err(GradienceError::InvalidCredential("invalid evm recipient address".into()));
        }
        Ok(X402Requirement {
            scheme: "exact".into(),
            network: net.into(),
            token_address: token_address.into(),
            recipient: recipient.into(),
            amount: amount.into(),
            deadline,
        })
    }

    pub fn sign_payment(
        &self,
        requirement: X402Requirement,
        signature: &str,
    ) -> Result<X402Payment> {
        if signature.len() < 10 {
            return Err(GradienceError::Signature("invalid signature length".into()));
        }
        Ok(X402Payment {
            requirement,
            signature: signature.into(),
            tx_hash: None,
        })
    }

    pub fn verify_receipt(&self, payment: &X402Payment, current_time: u64) -> Result<bool> {
        if current_time > payment.requirement.deadline {
            return Ok(false);
        }
        if payment.signature.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }

    /// Settle an x402 payment by constructing and broadcasting an ERC-20 transfer.
    /// Returns the on-chain transaction hash.
    pub async fn settle_payment(
        &self,
        payment: &mut X402Payment,
        wallet_id: &str,
        from_address: &str,
        chain: &str,
        passphrase: &str,
        vault_dir: &std::path::Path,
    ) -> Result<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if !self.verify_receipt(payment, now)? {
            return Err(GradienceError::Signature("x402 receipt invalid or expired".into()));
        }

        if payment.requirement.network.starts_with("stellar") {
            return Err(GradienceError::Validation(
                "Stellar settlement not yet implemented (requires Stellar signer integration)".into(),
            ));
        }

        let token = &payment.requirement.token_address;
        let to = payment.requirement.recipient.trim_start_matches("0x");
        let amount_hex = format!("{:064x}", payment.requirement.amount.parse::<u128>().unwrap_or(0));

        // ERC-20 transfer(address,uint256) selector = 0xa9059cbb
        let mut data = hex::decode("a9059cbb").unwrap();
        data.extend_from_slice(&hex::decode(format!("{:0>40}", to)).unwrap());
        data.extend_from_slice(&hex::decode(amount_hex).unwrap());

        let rpc_url = if chain == "base" {
            "https://mainnet.base.org"
        } else {
            "https://eth.llamarpc.com"
        };

        let client = crate::rpc::evm::EvmRpcClient::new("evm", rpc_url)
            .map_err(|e| GradienceError::Http(e.to_string()))?;
        let nonce = client
            .get_transaction_count(from_address)
            .await
            .map_err(|e| GradienceError::Http(e.to_string()))?;
        let gas_price_hex = client
            .get_gas_price()
            .await
            .map_err(|e| GradienceError::Http(e.to_string()))?;
        let gas_price = u128::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16)
            .map_err(|_| GradienceError::Validation("bad gas price".into()))?;

        let chain_num: u64 = if chain == "base" { 8453 } else { 1 };
        let token_address_bytes = hex::decode(token.trim_start_matches("0x")).unwrap_or_default();

        let mut rlp = rlp::RlpStream::new_list(9);
        rlp.append(&nonce);
        rlp.append(&gas_price);
        rlp.append(&100000u64); // gas limit for ERC-20 transfer
        rlp.append(&token_address_bytes);
        rlp.append(&0u128); // value = 0 (token transfer)
        rlp.append(&data);
        rlp.append(&chain_num);
        rlp.append(&0u8);
        rlp.append(&0u8);
        let tx_hex = format!("0x{}", hex::encode(&rlp.out()));

        let result = ows_lib::sign_and_send(
            wallet_id,
            chain,
            &tx_hex,
            Some(passphrase),
            None,
            Some(rpc_url),
            Some(vault_dir),
        )
        .map_err(|e| GradienceError::Ows(e.to_string()))?;

        payment.tx_hash = Some(result.tx_hash.clone());
        Ok(result.tx_hash)
    }
}
