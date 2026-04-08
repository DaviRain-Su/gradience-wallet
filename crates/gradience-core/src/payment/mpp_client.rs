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
}

impl GradienceMppProvider {
    pub fn new(wallet_id: &str, router: PaymentRouter) -> Self {
        Self {
            wallet_id: wallet_id.into(),
            router,
            policy_guard: false,
            tempo_signer: None,
            tempo_rpc: "https://rpc.moderato.tempo.xyz".into(),
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
}

impl PaymentProvider for GradienceMppProvider {
    fn supports(&self, method: &str, intent: &str) -> bool {
        (method == "tempo" && intent == "charge" && self.tempo_signer.is_some())
            || (method == "gradience" && intent == "session")
    }

    async fn pay(
        &self,
        challenge: &PaymentChallenge,
    ) -> Result<PaymentCredential, mpp::MppError> {
        use mpp::protocol::intents::ChargeRequest;

        if challenge.method.as_str() == "tempo" && challenge.intent.as_str() == "charge" {
            let signer = self
                .tempo_signer
                .clone()
                .ok_or_else(|| mpp::MppError::Http("missing tempo signer".into()))?;
            let tempo = mpp::client::TempoProvider::new(signer, &self.tempo_rpc)
                .map_err(|e| mpp::MppError::InvalidConfig(e.to_string()))?;
            return tempo.pay(challenge).await;
        }

        if challenge.method.as_str() == "gradience" && challenge.intent.as_str() == "session" {
            // Session credentials are handled out-of-band in the retry path.
            return Err(mpp::MppError::Http(
                "session credentials must be attached manually".into(),
            ));
        }

        // 1. Parse charge request details
        let charge_req: ChargeRequest = challenge
            .request
            .decode()
            .map_err(|e| mpp::MppError::InvalidConfig(format!("decode charge request: {}", e)))?;

        let _amount = charge_req
            .parse_amount()
            .map_err(|e| mpp::MppError::InvalidConfig(format!("invalid amount: {}", e)))?;

        // 2. Multi-chain routing (even for Tempo we route through the preference list)
        let _route = self
            .router
            .select_route(&PaymentRequirement {
                amount: charge_req.amount,
                token_hint: Some(charge_req.currency),
            })
            .await
            .map_err(|e| mpp::MppError::Http(e.to_string()))?;

        Err(mpp::MppError::Http(
            "unsupported payment method for this wallet".into(),
        ))
    }
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
