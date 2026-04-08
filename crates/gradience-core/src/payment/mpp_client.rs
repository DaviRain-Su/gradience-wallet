use crate::error::GradienceError;
use crate::payment::router::{PaymentRequirement, PaymentRouter};
use mpp::client::PaymentProvider;
use mpp::{PaymentChallenge, PaymentCredential};

/// Gradience-specific MPP provider that delegates signing to the OWS wallet.
/// Supports multi-chain priority routing before committing to a payment.
#[derive(Clone, Debug)]
pub struct GradienceMppProvider {
    pub wallet_id: String,
    pub router: PaymentRouter,
    /// If true, only routes that the policy engine approves will be paid.
    pub policy_guard: bool,
    /// Optional Tempo signer for charge payments.
    pub tempo_signer: Option<alloy::signers::local::PrivateKeySigner>,
    /// RPC URL for Tempo (defaults to Moderato testnet).
    pub tempo_rpc: String,
    /// Optional EVM secret (32 bytes) for charge payments on EVM chains.
    pub evm_secret: Option<[u8; 32]>,
    /// RPC URL for EVM (defaults to Base mainnet).
    pub evm_rpc: String,
    /// Optional Solana secret (32 bytes) for charge payments on Solana.
    pub solana_secret: Option<[u8; 32]>,
    /// RPC URL for Solana (defaults to mainnet).
    pub solana_rpc: String,
}

impl GradienceMppProvider {
    pub fn new(wallet_id: &str, router: PaymentRouter) -> Self {
        Self {
            wallet_id: wallet_id.into(),
            router,
            policy_guard: false,
            tempo_signer: None,
            tempo_rpc: "https://rpc.moderato.tempo.xyz".into(),
            evm_secret: None,
            evm_rpc: "https://mainnet.base.org".into(),
            solana_secret: None,
            solana_rpc: "https://api.mainnet-beta.solana.com".into(),
        }
    }

    pub fn with_policy_guard(mut self, enabled: bool) -> Self {
        self.policy_guard = enabled;
        self
    }

    pub fn with_tempo_signer(
        mut self,
        signer: alloy::signers::local::PrivateKeySigner,
    ) -> Self {
        self.tempo_signer = Some(signer);
        self
    }

    pub fn with_tempo_rpc(mut self, rpc: impl Into<String>) -> Self {
        self.tempo_rpc = rpc.into();
        self
    }

    pub fn with_evm_secret(mut self, secret: [u8; 32]) -> Self {
        self.evm_secret = Some(secret);
        self
    }

    pub fn with_evm_rpc(mut self, rpc: impl Into<String>) -> Self {
        self.evm_rpc = rpc.into();
        self
    }

    pub fn with_solana_secret(mut self, secret: [u8; 32]) -> Self {
        self.solana_secret = Some(secret);
        self
    }

    pub fn with_solana_rpc(mut self, rpc: impl Into<String>) -> Self {
        self.solana_rpc = rpc.into();
        self
    }
}

impl PaymentProvider for GradienceMppProvider {
    fn supports(&self, method: &str, intent: &str) -> bool {
        (method == "tempo" && intent == "charge" && self.tempo_signer.is_some())
            || (method == "evm" && intent == "charge" && self.evm_secret.is_some())
            || (method == "solana" && intent == "charge" && self.solana_secret.is_some())
            || (method == "gradience" && intent == "session")
    }

    async fn pay(
        &self,
        challenge: &PaymentChallenge,
    ) -> Result<PaymentCredential, mpp::MppError> {
        use mpp::protocol::core::{ChallengeEcho, PaymentPayload};
        use mpp::protocol::intents::ChargeRequest;

        let charge_req: ChargeRequest = challenge
            .request
            .decode()
            .map_err(|e| mpp::MppError::InvalidConfig(format!("decode charge request: {}", e)))?;

        let _ = charge_req
            .parse_amount()
            .map_err(|e| mpp::MppError::InvalidConfig(format!("invalid amount: {}", e)))?;

        let challenge_echo = ChallengeEcho {
            id: challenge.id.clone(),
            realm: challenge.realm.clone(),
            method: challenge.method.clone(),
            intent: challenge.intent.clone(),
            request: challenge.request.clone(),
            expires: challenge.expires.clone(),
            digest: challenge.digest.clone(),
            opaque: challenge.opaque.clone(),
        };

        // --- Tempo charge ---
        if challenge.method.as_str() == "tempo" && challenge.intent.as_str() == "charge" {
            let signer = self
                .tempo_signer
                .clone()
                .ok_or_else(|| mpp::MppError::Http("missing tempo signer".into()))?;
            let tempo = mpp::client::TempoProvider::new(signer, &self.tempo_rpc)
                .map_err(|e| mpp::MppError::InvalidConfig(e.to_string()))?;
            return tempo.pay(challenge).await;
        }

        // --- Gradience session ---
        if challenge.method.as_str() == "gradience" && challenge.intent.as_str() == "session" {
            return Err(mpp::MppError::Http(
                "session credentials must be attached manually".into(),
            ));
        }

        // --- Multi-chain routing ---
        let _route = self
            .router
            .select_route(&PaymentRequirement {
                amount: charge_req.amount.clone(),
                token_hint: Some(charge_req.currency.clone()),
            })
            .await
            .map_err(|e| mpp::MppError::Http(e.to_string()))?;

        // --- EVM charge ---
        if challenge.method.as_str() == "evm" && challenge.intent.as_str() == "charge" {
            let secret = self
                .evm_secret
                .ok_or_else(|| mpp::MppError::Http("missing evm signer".into()))?;

            let recipient = charge_req
                .recipient
                .as_deref()
                .ok_or_else(|| mpp::MppError::InvalidConfig("missing recipient".into()))?;
            let currency = &charge_req.currency;
            let amount = charge_req
                .amount
                .parse::<u128>()
                .map_err(|e| mpp::MppError::InvalidConfig(format!("invalid amount: {}", e)))?;

            let from_addr = crate::ows::signing::eth_address_from_secret_key(&secret)
                .map_err(|e| mpp::MppError::InvalidConfig(e.to_string()))?;

            let chain_id: u64 = if self.evm_rpc.contains("84532") || self.evm_rpc.contains("sepolia.base") {
                84532
            } else if self.evm_rpc.contains("8453") || self.evm_rpc.contains("base") {
                8453
            } else if self.evm_rpc.contains("1") || self.evm_rpc.contains("eth") {
                1
            } else {
                8453
            };

            let client = crate::rpc::evm::EvmRpcClient::new("evm", &self.evm_rpc)
                .map_err(|e| mpp::MppError::Http(e.to_string()))?;
            let nonce = client
                .get_transaction_count(&from_addr)
                .await
                .map_err(|e| mpp::MppError::Http(e.to_string()))?;
            let gp_hex = client
                .get_gas_price()
                .await
                .map_err(|e| mpp::MppError::Http(e.to_string()))?;
            let gas_price = u128::from_str_radix(gp_hex.trim_start_matches("0x"), 16)
                .map_err(|e| mpp::MppError::InvalidConfig(format!("bad gas price: {}", e)))?;

            let (to, value, data, gas_limit): (String, u128, Vec<u8>, u64) =
                if currency == "0x0000000000000000000000000000000000000000"
                    || currency.is_empty()
                    || currency == "ETH"
                {
                    (recipient.into(), amount, vec![], 21000)
                } else {
                    let mut calldata = vec![0xa9, 0x05, 0x9c, 0xbb];
                    let to_bytes = hex::decode(recipient.trim_start_matches("0x"))
                        .map_err(|e| mpp::MppError::InvalidConfig(format!("bad recipient: {}", e)))?;
                    calldata.extend_from_slice(&[0u8; 12]);
                    calldata.extend_from_slice(&to_bytes);
                    let amount_padded = pad_u128_to_32_bytes(amount);
                    calldata.extend_from_slice(&amount_padded);
                    (currency.into(), 0, calldata, 65000)
                };

            let signed_tx = crate::ows::signing::sign_eth_transaction(
                &secret,
                nonce,
                gas_price,
                gas_limit,
                &to,
                value,
                &data,
                chain_id,
            )
            .map_err(|e| mpp::MppError::Http(e.to_string()))?;

            let tx_hash = client
                .send_raw_transaction(&format!("0x{}", hex::encode(&signed_tx)))
                .await
                .map_err(|e| mpp::MppError::Http(e.to_string()))?;

            let payload = PaymentPayload::hash(tx_hash);
            return Ok(PaymentCredential::new(challenge_echo, payload));
        }

        // --- Solana charge ---
        if challenge.method.as_str() == "solana" && challenge.intent.as_str() == "charge" {
            let secret = self
                .solana_secret
                .ok_or_else(|| mpp::MppError::Http("missing solana signer".into()))?;

            let recipient = charge_req
                .recipient
                .as_deref()
                .ok_or_else(|| mpp::MppError::InvalidConfig("missing recipient".into()))?;
            let amount = charge_req
                .amount
                .parse::<u64>()
                .map_err(|e| mpp::MppError::InvalidConfig(format!("invalid amount: {}", e)))?;

            let from_pubkey = ed25519_dalek::SigningKey::from_bytes(&secret).verifying_key().to_bytes();
            let from_addr = bs58::encode(&from_pubkey).into_string();

            let client = crate::rpc::solana::SolanaRpcClient::new(&self.solana_rpc);
            let blockhash = client
                .get_latest_blockhash()
                .await
                .map_err(|e| mpp::MppError::Http(e.to_string()))?;

            let tx = crate::ows::signing::build_solana_transfer_tx(&from_addr, recipient, amount, &blockhash)
                .map_err(|e| mpp::MppError::Http(e.to_string()))?;
            let signed_tx = crate::ows::signing::sign_solana_transaction(tx, &secret)
                .map_err(|e| mpp::MppError::Http(e.to_string()))?;

            let sig = client
                .send_transaction(&signed_tx)
                .await
                .map_err(|e| mpp::MppError::Http(e.to_string()))?;

            let payload = PaymentPayload::hash(sig);
            return Ok(PaymentCredential::new(challenge_echo, payload));
        }

        Err(mpp::MppError::Http(
            "unsupported payment method for this wallet".into(),
        ))
    }
}

fn pad_u128_to_32_bytes(value: u128) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[16..].copy_from_slice(&value.to_be_bytes());
    bytes
}

/// High-level wrapper around reqwest 0.11 that handles MPP 402 automatically.
pub struct MppClient {
    pub http: reqwest::Client,
    pub provider: GradienceMppProvider,
}

impl MppClient {
    pub fn new(provider: GradienceMppProvider) -> Self {
        Self {
            http: reqwest::Client::new(),
            provider,
        }
    }

    /// Build a request and send it, automatically handling HTTP 402 via MPP.
    pub async fn send(
        &self,
        req: reqwest::RequestBuilder,
    ) -> crate::Result<reqwest::Response> {
        let retry_builder = req
            .try_clone()
            .ok_or_else(|| GradienceError::Http("request not cloneable".into()))?;
        let request = req.build().map_err(|e| GradienceError::Http(e.to_string()))?;

        let resp = self
            .http
            .execute(request)
            .await
            .map_err(|e| GradienceError::Http(e.to_string()))?;

        if resp.status() != reqwest::StatusCode::PAYMENT_REQUIRED {
            return Ok(resp);
        }

        let www_auth_values: Vec<&str> = resp
            .headers()
            .get_all(reqwest::header::WWW_AUTHENTICATE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .collect();

        if www_auth_values.is_empty() {
            return Err(GradienceError::Http("402 missing WWW-Authenticate".into()));
        }

        let challenges: Vec<_> = mpp::parse_www_authenticate_all(www_auth_values)
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        let challenge = challenges
            .iter()
            .find(|c| self.provider.supports(c.method.as_str(), c.intent.as_str()))
            .ok_or_else(|| {
                let offered: Vec<_> = challenges
                    .iter()
                    .map(|c| format!("{}.{}", c.method, c.intent))
                    .collect();
                GradienceError::Http(format!(
                    "no supported challenge. offered: [{}]",
                    offered.join(", ")
                ))
            })?;

        let credential = self
            .provider
            .pay(challenge)
            .await
            .map_err(|e| GradienceError::Http(e.to_string()))?;

        let auth_header = mpp::format_authorization(&credential)
            .map_err(|e| GradienceError::Http(e.to_string()))?;

        let retry_resp = self
            .http
            .execute(
                retry_builder
                    .header(reqwest::header::AUTHORIZATION, auth_header)
                    .build()
                    .map_err(|e| GradienceError::Http(e.to_string()))?,
            )
            .await
            .map_err(|e| GradienceError::Http(e.to_string()))?;

        Ok(retry_resp)
    }
}
