use crate::error::GradienceError;
use crate::payment::router::{PaymentRequirement, PaymentRouter};
use mpp::client::PaymentProvider;
use mpp::{PaymentChallenge, PaymentCredential};

/// Per-chain EVM configuration for MPP charge payments.
#[derive(Clone, Debug)]
pub struct EvmChargeConfig {
    pub chain_id: u64,
    pub rpc_url: String,
    pub secret: [u8; 32],
    pub gas_limit_native: u64,
    pub gas_limit_erc20: u64,
}

impl EvmChargeConfig {
    pub fn new(chain_id: u64, rpc_url: impl Into<String>, secret: [u8; 32]) -> Self {
        Self {
            chain_id,
            rpc_url: rpc_url.into(),
            secret,
            gas_limit_native: 21000,
            gas_limit_erc20: 65000,
        }
    }

    pub fn with_gas_limits(mut self, native: u64, erc20: u64) -> Self {
        self.gas_limit_native = native;
        self.gas_limit_erc20 = erc20;
        self
    }
}

/// Gradience-specific MPP provider that delegates signing to the OWS wallet.
/// Supports multi-chain priority routing before committing to a payment.
#[derive(Clone, Debug)]
pub struct GradienceMppProvider {
    pub wallet_id: String,
    pub router: PaymentRouter,
    pub policy_guard: bool,
    pub tempo_signer: Option<alloy::signers::local::PrivateKeySigner>,
    pub tempo_rpc: String,
    /// Multi-chain EVM configs keyed by chain_id.
    pub evm_chains: Vec<EvmChargeConfig>,
    pub solana_secret: Option<[u8; 32]>,
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
            evm_chains: Vec::new(),
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

    /// Add an EVM chain config for MPP charge payments.
    pub fn with_evm_chain(mut self, config: EvmChargeConfig) -> Self {
        self.evm_chains.push(config);
        self
    }

    /// Convenience: add a single EVM chain with just secret and chain name.
    /// RPC and chain_id are resolved from `chain.rs`.
    pub fn with_evm_secret(mut self, secret: [u8; 32]) -> Self {
        self.evm_chains.push(EvmChargeConfig::new(
            8453,
            "https://mainnet.base.org",
            secret,
        ));
        self
    }

    /// Convenience: set the RPC for the first (or only) EVM chain.
    pub fn with_evm_rpc(mut self, rpc: impl Into<String>) -> Self {
        let rpc_str = rpc.into();
        if let Some(first) = self.evm_chains.first_mut() {
            first.rpc_url = rpc_str;
        }
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

    /// Find the best EVM chain config for a given charge request.
    /// If the challenge specifies a chain, use that; otherwise pick the first available.
    fn find_evm_config(&self, chain_hint: Option<u64>) -> Option<&EvmChargeConfig> {
        if let Some(target) = chain_hint {
            self.evm_chains.iter().find(|c| c.chain_id == target)
        } else {
            self.evm_chains.first()
        }
    }

    async fn pay_evm_charge(
        &self,
        charge_req: &mpp::protocol::intents::ChargeRequest,
        challenge_echo: mpp::protocol::core::ChallengeEcho,
    ) -> Result<PaymentCredential, mpp::MppError> {
        use mpp::protocol::core::PaymentPayload;

        // Extract chain hint from methodDetails.chainId if present.
        let chain_hint: Option<u64> = charge_req
            .method_details
            .as_ref()
            .and_then(|d| d.get("chainId"))
            .and_then(|v| v.as_u64());

        let evm = self
            .find_evm_config(chain_hint)
            .ok_or_else(|| mpp::MppError::Http("no evm chain configured".into()))?;

        let recipient = charge_req
            .recipient
            .as_deref()
            .ok_or_else(|| mpp::MppError::InvalidConfig("missing recipient".into()))?;
        let currency = &charge_req.currency;
        let amount = charge_req
            .amount
            .parse::<u128>()
            .map_err(|e| mpp::MppError::InvalidConfig(format!("invalid amount: {}", e)))?;

        let from_addr = crate::ows::signing::eth_address_from_secret_key(&evm.secret)
            .map_err(|e| mpp::MppError::InvalidConfig(e.to_string()))?;

        let client = crate::rpc::evm::EvmRpcClient::new("evm", &evm.rpc_url)
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

        let is_native = currency == "0x0000000000000000000000000000000000000000"
            || currency.is_empty()
            || currency == "ETH"
            || currency == "BNB"
            || currency == "CFX"
            || currency == "OKB";

        let (to, value, data, gas_limit): (String, u128, Vec<u8>, u64) = if is_native {
            (recipient.into(), amount, vec![], evm.gas_limit_native)
        } else {
            let mut calldata = vec![0xa9, 0x05, 0x9c, 0xbb];
            let to_bytes = hex::decode(recipient.trim_start_matches("0x"))
                .map_err(|e| mpp::MppError::InvalidConfig(format!("bad recipient: {}", e)))?;
            calldata.extend_from_slice(&[0u8; 12]);
            calldata.extend_from_slice(&to_bytes);
            calldata.extend_from_slice(&pad_u128_to_32_bytes(amount));
            (currency.into(), 0, calldata, evm.gas_limit_erc20)
        };

        let signed_tx = crate::ows::signing::sign_eth_transaction(
            &evm.secret,
            nonce,
            gas_price,
            gas_limit,
            &to,
            value,
            &data,
            evm.chain_id,
        )
        .map_err(|e| mpp::MppError::Http(e.to_string()))?;

        let tx_hash = client
            .send_raw_transaction(&format!("0x{}", hex::encode(&signed_tx)))
            .await
            .map_err(|e| mpp::MppError::Http(e.to_string()))?;

        let payload = PaymentPayload::hash(tx_hash);
        Ok(PaymentCredential::new(challenge_echo, payload))
    }

    async fn pay_solana_charge(
        &self,
        charge_req: &mpp::protocol::intents::ChargeRequest,
        challenge_echo: mpp::protocol::core::ChallengeEcho,
    ) -> Result<PaymentCredential, mpp::MppError> {
        use mpp::protocol::core::PaymentPayload;

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

        let from_pubkey = ed25519_dalek::SigningKey::from_bytes(&secret)
            .verifying_key()
            .to_bytes();
        let from_addr = bs58::encode(&from_pubkey).into_string();

        let client = crate::rpc::solana::SolanaRpcClient::new(&self.solana_rpc);
        let blockhash = client
            .get_latest_blockhash()
            .await
            .map_err(|e| mpp::MppError::Http(e.to_string()))?;

        let tx = crate::ows::signing::build_solana_transfer_tx(
            &from_addr, recipient, amount, &blockhash,
        )
        .map_err(|e| mpp::MppError::Http(e.to_string()))?;
        let signed_tx = crate::ows::signing::sign_solana_transaction(tx, &secret)
            .map_err(|e| mpp::MppError::Http(e.to_string()))?;

        let sig = client
            .send_transaction(&signed_tx)
            .await
            .map_err(|e| mpp::MppError::Http(e.to_string()))?;

        let payload = PaymentPayload::hash(sig);
        Ok(PaymentCredential::new(challenge_echo, payload))
    }
}

impl PaymentProvider for GradienceMppProvider {
    fn supports(&self, method: &str, intent: &str) -> bool {
        (method == "tempo" && intent == "charge" && self.tempo_signer.is_some())
            || (method == "evm" && intent == "charge" && !self.evm_chains.is_empty())
            || (method == "solana" && intent == "charge" && self.solana_secret.is_some())
            || (method == "gradience" && intent == "session")
    }

    async fn pay(
        &self,
        challenge: &PaymentChallenge,
    ) -> Result<PaymentCredential, mpp::MppError> {
        use mpp::protocol::core::ChallengeEcho;
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

        // --- EVM charge (multi-chain) ---
        if challenge.method.as_str() == "evm" && challenge.intent.as_str() == "charge" {
            return self.pay_evm_charge(&charge_req, challenge_echo).await;
        }

        // --- Solana charge ---
        if challenge.method.as_str() == "solana" && challenge.intent.as_str() == "charge" {
            return self.pay_solana_charge(&charge_req, challenge_echo).await;
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
