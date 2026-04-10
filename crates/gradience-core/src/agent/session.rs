use crate::error::{GradienceError, Result};
use chrono::{DateTime, Utc};
use sqlx::{Pool, Sqlite};

#[derive(Debug, Clone)]
pub enum SessionType {
    CapabilityToken,
    OnChainSessionKey,
}

impl std::fmt::Display for SessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionType::CapabilityToken => write!(f, "capability_token"),
            SessionType::OnChainSessionKey => write!(f, "on_chain_session_key"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpendLimit {
    pub limit_type: String, // per_tx | daily | total
    pub token: String,
    pub amount_raw: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionBoundaries {
    pub allowed_chains: Vec<String>,
    pub allowed_actions: Vec<String>,
    pub spend_limits: Vec<SpendLimit>,
    pub contract_whitelist: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub enum SessionCredential {
    Token(String),
    Signer(alloy::signers::local::PrivateKeySigner),
}

pub struct AgentSessionService;

impl Default for AgentSessionService {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentSessionService {
    pub fn new() -> Self {
        Self
    }

    /// Convert an AgentSession into a temporary Policy for the PolicyEngine.
    pub fn to_policy(&self, session: &gradience_db::models::AgentSession) -> Result<Option<crate::policy::engine::Policy>> {
        let boundaries: SessionBoundaries = match session.boundaries_json.as_deref() {
            Some(json) => serde_json::from_str(json)
                .map_err(|e| GradienceError::Database(format!("parse boundaries: {}", e)))?,
            None => return Ok(None),
        };

        let mut rules = Vec::new();

        if !boundaries.allowed_chains.is_empty() {
            rules.push(crate::policy::engine::Rule::ChainWhitelist {
                chain_ids: boundaries.allowed_chains.clone(),
            });
        }

        if !boundaries.allowed_actions.is_empty() {
            rules.push(crate::policy::engine::Rule::OperationType {
                allowed: boundaries.allowed_actions.clone(),
            });
        }

        if let Some(contracts) = boundaries.contract_whitelist {
            if !contracts.is_empty() {
                rules.push(crate::policy::engine::Rule::ContractWhitelist {
                    contracts,
                });
            }
        }

        for limit in &boundaries.spend_limits {
            if limit.limit_type == "per_tx" {
                rules.push(crate::policy::engine::Rule::SpendLimit {
                    max: limit.amount_raw.clone(),
                    token: limit.token.clone(),
                });
            }
        }

        if rules.is_empty() {
            return Ok(None);
        }

        Ok(Some(crate::policy::engine::Policy {
            id: format!("session-{}", session.id),
            name: format!("Agent Session {}", session.name),
            wallet_id: Some(session.wallet_id.clone()),
            workspace_id: None,
            rules,
            priority: 100, // session policy wins over most wallet policies
            status: session.status.clone(),
            version: 1,
            created_at: session.created_at.to_rfc3339(),
            updated_at: session.created_at.to_rfc3339(),
        }))
    }

    pub async fn create_session(
        &self,
        pool: &Pool<Sqlite>,
        wallet_id: &str,
        name: &str,
        session_type: SessionType,
        boundaries: SessionBoundaries,
        expires_at: DateTime<Utc>,
    ) -> Result<(String, SessionCredential)> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let boundaries_json = serde_json::to_string(&boundaries)
            .map_err(|e| GradienceError::Database(format!("serialize boundaries: {}", e)))?;

        let (agent_key_hash, credential) = match session_type {
            SessionType::CapabilityToken => {
                let token = uuid::Uuid::new_v4().to_string();
                let hash = ring::digest::digest(&ring::digest::SHA256, token.as_bytes());
                (Some(hex::encode(hash.as_ref())), SessionCredential::Token(token))
            }
            SessionType::OnChainSessionKey => {
                let signer = alloy::signers::local::PrivateKeySigner::random();
                let pubkey = hex::encode(signer.address());
                (Some(pubkey), SessionCredential::Signer(signer))
            }
        };

        gradience_db::queries::create_agent_session(
            pool,
            &session_id,
            wallet_id,
            name,
            &session_type.to_string(),
            agent_key_hash.as_deref(),
            "active",
            expires_at,
            Some(&boundaries_json),
        )
        .await
        .map_err(|e| GradienceError::Database(format!("create agent session failed: {}", e)))?;

        for limit in &boundaries.spend_limits {
            gradience_db::queries::create_agent_session_limit(
                pool,
                &session_id,
                &limit.limit_type,
                &limit.token,
                &limit.amount_raw,
            )
            .await
            .map_err(|e| {
                GradienceError::Database(format!("create agent session limit failed: {}", e))
            })?;
        }

        Ok((session_id, credential))
    }

    pub async fn validate_session(
        &self,
        pool: &Pool<Sqlite>,
        session_id: &str,
        action: &str,
        chain_id: &str,
        _contract: Option<&str>,
    ) -> Result<gradience_db::models::AgentSession> {
        let session = gradience_db::queries::get_agent_session_by_id(pool, session_id)
            .await
            .map_err(|e| GradienceError::Database(format!("db error: {}", e)))?
            .ok_or_else(|| GradienceError::Validation("agent session not found".into()))?;

        if session.status != "active" {
            return Err(GradienceError::Validation(
                "agent session is not active".into(),
            ));
        }

        if session.expires_at < Utc::now() {
            return Err(GradienceError::Validation("agent session expired".into()));
        }

        if let Some(ref json) = session.boundaries_json {
            let boundaries: SessionBoundaries = serde_json::from_str(json)
                .map_err(|e| GradienceError::Database(format!("parse boundaries: {}", e)))?;

            if !boundaries.allowed_chains.is_empty()
                && !boundaries.allowed_chains.iter().any(|c| chain_id.starts_with(c))
            {
                return Err(GradienceError::Validation(format!(
                    "chain {} not allowed by session",
                    chain_id
                )));
            }

            if !boundaries.allowed_actions.is_empty()
                && !boundaries.allowed_actions.iter().any(|a| action.eq_ignore_ascii_case(a))
            {
                return Err(GradienceError::Validation(format!(
                    "action {} not allowed by session",
                    action
                )));
            }
        }

        Ok(session)
    }

    pub async fn consume_budget(
        &self,
        pool: &Pool<Sqlite>,
        session_id: &str,
        token: &str,
        amount_raw: &str,
    ) -> Result<()> {
        let amount: u128 = amount_raw
            .parse()
            .map_err(|_| GradienceError::Validation("invalid amount".into()))?;

        let limits = gradience_db::queries::get_agent_session_limits(pool, session_id)
            .await
            .map_err(|e| GradienceError::Database(format!("db error: {}", e)))?;

        let today = Utc::now().date_naive().to_string();

        for limit in limits {
            if limit.token != token {
                continue;
            }
            let limit_val: u128 = limit
                .amount_raw
                .parse()
                .unwrap_or(u128::MAX);

            match limit.limit_type.as_str() {
                "per_tx" => {
                    if amount > limit_val {
                        return Err(GradienceError::Validation(format!(
                            "per_tx limit exceeded: {} > {}",
                            amount_raw, limit.amount_raw
                        )));
                    }
                }
                "daily" => {
                    let usage =
                        gradience_db::queries::get_agent_session_usage(
                            pool, session_id, token, &today,
                        )
                        .await
                        .map_err(|e| GradienceError::Database(format!("db error: {}", e)))?;

                    let current: u128 = usage
                        .map(|u| u.spent_raw.parse().unwrap_or(0u128))
                        .unwrap_or(0u128);

                    if current + amount > limit_val {
                        return Err(GradienceError::Validation(format!(
                            "daily limit exceeded: {} + {} > {}",
                            current, amount_raw, limit.amount_raw
                        )));
                    }

                    let new_spent = (current + amount).to_string();
                    gradience_db::queries::upsert_agent_session_usage(
                        pool, session_id, token, &today, &new_spent,
                    )
                    .await
                    .map_err(|e| GradienceError::Database(format!("db error: {}", e)))?;
                }
                "total" => {
                    let usage = gradience_db::queries::get_agent_session_usage(
                        pool,
                        session_id,
                        token,
                        "1970-01-01",
                    )
                    .await
                    .map_err(|e| GradienceError::Database(format!("db error: {}", e)))?;

                    let current: u128 = usage
                        .map(|u| u.spent_raw.parse().unwrap_or(0u128))
                        .unwrap_or(0u128);

                    if current + amount > limit_val {
                        return Err(GradienceError::Validation(format!(
                            "total limit exceeded: {} + {} > {}",
                            current, amount_raw, limit.amount_raw
                        )));
                    }

                    let new_spent = (current + amount).to_string();
                    gradience_db::queries::upsert_agent_session_usage(
                        pool,
                        session_id,
                        token,
                        "1970-01-01",
                        &new_spent,
                    )
                    .await
                    .map_err(|e| GradienceError::Database(format!("db error: {}", e)))?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn revoke_session(&self, pool: &Pool<Sqlite>, session_id: &str) -> Result<()> {
        gradience_db::queries::revoke_agent_session(pool, session_id)
            .await
            .map_err(|e| GradienceError::Database(format!("db error: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> Pool<Sqlite> {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../gradience-db/migrations")
            .run(&pool)
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_agent_session_lifecycle() {
        let pool = setup_db().await;

        // need a wallet first
        gradience_db::queries::create_user(&pool, "u1", "a@b.com").await.unwrap();
        gradience_db::queries::create_wallet(&pool, "w1", "main", "u1", None::<&str>)
            .await
            .unwrap();

        let svc = AgentSessionService::new();
        let boundaries = SessionBoundaries {
            allowed_chains: vec!["eip155:8453".into()],
            allowed_actions: vec!["transfer".into(), "swap".into()],
            spend_limits: vec![
                SpendLimit {
                    limit_type: "per_tx".into(),
                    token: "ETH".into(),
                    amount_raw: "1000000000000000000".into(), // 1 ETH
                },
                SpendLimit {
                    limit_type: "daily".into(),
                    token: "ETH".into(),
                    amount_raw: "5000000000000000000".into(), // 5 ETH
                },
            ],
            contract_whitelist: None,
        };

        let (session_id, cred) = svc
            .create_session(
                &pool,
                "w1",
                "test-agent",
                SessionType::CapabilityToken,
                boundaries,
                Utc::now() + chrono::Duration::hours(24),
            )
            .await
            .unwrap();

        assert!(matches!(cred, SessionCredential::Token(_)));

        // validate ok
        svc.validate_session(&pool, &session_id, "transfer", "eip155:8453", None)
            .await
            .unwrap();

        // validate wrong chain
        let err = svc
            .validate_session(&pool, &session_id, "transfer", "solana:101", None)
            .await;
        assert!(err.is_err());

        // consume budget
        svc.consume_budget(&pool, &session_id, "ETH", "500000000000000000")
            .await
            .unwrap();

        // exceed per_tx
        let err = svc
            .consume_budget(&pool, &session_id, "ETH", "2000000000000000000")
            .await;
        assert!(err.is_err());

        // revoke
        svc.revoke_session(&pool, &session_id).await.unwrap();
        let err = svc
            .validate_session(&pool, &session_id, "transfer", "eip155:8453", None)
            .await;
        assert!(err.is_err());
    }
}
