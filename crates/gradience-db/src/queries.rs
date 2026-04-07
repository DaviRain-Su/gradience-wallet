use crate::models::*;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Sqlite};

// ========== Users ==========
pub async fn create_user(pool: &Pool<Sqlite>, id: &str, email: &str) -> Result<()> {
    sqlx::query!(
        "INSERT INTO users (id, email) VALUES (?, ?)",
        id,
        email
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_user_by_email(pool: &Pool<Sqlite>, email: &str) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, email, created_at, updated_at, status FROM users WHERE email = ?"
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

// ========== Wallets ==========
pub async fn create_wallet(
    pool: &Pool<Sqlite>,
    id: &str,
    name: &str,
    owner_id: &str,
    workspace_id: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO wallets (id, name, owner_id, workspace_id) VALUES (?, ?, ?, ?)",
        id,
        name,
        owner_id,
        workspace_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_wallets_by_owner(pool: &Pool<Sqlite>, owner_id: &str) -> Result<Vec<Wallet>> {
    let wallets = sqlx::query_as::<_, Wallet>(
        "SELECT id, name, owner_id, workspace_id, status, created_at, updated_at FROM wallets WHERE owner_id = ? ORDER BY created_at DESC"
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await?;
    Ok(wallets)
}

pub async fn get_wallet_by_id(pool: &Pool<Sqlite>, id: &str) -> Result<Option<Wallet>> {
    let wallet = sqlx::query_as::<_, Wallet>(
        "SELECT id, name, owner_id, workspace_id, status, created_at, updated_at FROM wallets WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(wallet)
}

// ========== Wallet Addresses ==========
pub async fn create_wallet_address(
    pool: &Pool<Sqlite>,
    id: &str,
    wallet_id: &str,
    chain_id: &str,
    address: &str,
    derivation_path: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO wallet_addresses (id, wallet_id, chain_id, address, derivation_path) VALUES (?, ?, ?, ?, ?)",
        id,
        wallet_id,
        chain_id,
        address,
        derivation_path
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_wallet_addresses(pool: &Pool<Sqlite>, wallet_id: &str) -> Result<Vec<WalletAddress>> {
    let addresses = sqlx::query_as::<_, WalletAddress>(
        "SELECT id, wallet_id, chain_id, address, derivation_path FROM wallet_addresses WHERE wallet_id = ?"
    )
    .bind(wallet_id)
    .fetch_all(pool)
    .await?;
    Ok(addresses)
}

// ========== API Keys ==========
pub async fn create_api_key(
    pool: &Pool<Sqlite>,
    id: &str,
    wallet_id: &str,
    name: &str,
    key_hash: &[u8],
    permissions: &str,
    expires_at: Option<DateTime<Utc>>,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO api_keys (id, wallet_id, name, key_hash, permissions, expires_at) VALUES (?, ?, ?, ?, ?, ?)",
        id,
        wallet_id,
        name,
        key_hash,
        permissions,
        expires_at
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_api_key_by_hash(pool: &Pool<Sqlite>, hash: &[u8]) -> Result<Option<ApiKey>> {
    let key = sqlx::query_as::<_, ApiKey>(
        "SELECT id, wallet_id, name, key_hash, permissions, expires_at, last_used_at, created_at FROM api_keys WHERE key_hash = ? AND expires_at IS NULL"
    )
    .bind(hash)
    .fetch_optional(pool)
    .await?;
    Ok(key)
}

pub async fn revoke_api_key(pool: &Pool<Sqlite>, id: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE api_keys SET expires_at = datetime('now') WHERE id = ?",
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}

// ========== Policies ==========
pub async fn create_policy(
    pool: &Pool<Sqlite>,
    id: &str,
    name: &str,
    wallet_id: Option<&str>,
    workspace_id: Option<&str>,
    rules_json: &str,
    priority: i32,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO policies (id, name, wallet_id, workspace_id, rules_json, priority) VALUES (?, ?, ?, ?, ?, ?)",
        id,
        name,
        wallet_id,
        workspace_id,
        rules_json,
        priority
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_active_policies_by_wallet(pool: &Pool<Sqlite>, wallet_id: &str) -> Result<Vec<Policy>> {
    let policies = sqlx::query_as::<_, Policy>(
        "SELECT id, name, wallet_id, workspace_id, rules_json, priority, status, version, created_at, updated_at FROM policies WHERE wallet_id = ? AND status = 'active' ORDER BY priority DESC"
    )
    .bind(wallet_id)
    .fetch_all(pool)
    .await?;
    Ok(policies)
}

pub async fn list_active_policies_by_workspace(pool: &Pool<Sqlite>, workspace_id: &str) -> Result<Vec<Policy>> {
    let policies = sqlx::query_as::<_, Policy>(
        "SELECT id, name, wallet_id, workspace_id, rules_json, priority, status, version, created_at, updated_at FROM policies WHERE workspace_id = ? AND status = 'active' ORDER BY priority DESC"
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;
    Ok(policies)
}

// ========== Audit Logs ==========
pub async fn insert_audit_log(
    pool: &Pool<Sqlite>,
    wallet_id: &str,
    api_key_id: Option<&str>,
    action: &str,
    context_json: &str,
    decision: &str,
    prev_hash: &str,
    current_hash: &str,
) -> Result<i64> {
    let row = sqlx::query!(
        "INSERT INTO audit_logs (wallet_id, api_key_id, action, context_json, decision, prev_hash, current_hash) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
        wallet_id,
        api_key_id,
        action,
        context_json,
        decision,
        prev_hash,
        current_hash
    )
    .fetch_one(pool)
    .await?;
    Ok(row.id)
}

pub async fn list_audit_logs_by_wallet(pool: &Pool<Sqlite>, wallet_id: &str, limit: i64) -> Result<Vec<AuditLog>> {
    let logs = sqlx::query_as::<_, AuditLog>(
        "SELECT id, wallet_id, api_key_id, action, context_json, intent_json, decision, decision_reason, dynamic_factors, tx_hash, anchor_tx_hash, anchor_root, prev_hash, current_hash, created_at FROM audit_logs WHERE wallet_id = ? ORDER BY created_at DESC LIMIT ?"
    )
    .bind(wallet_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(logs)
}

// ========== Spending Tracker ==========
pub async fn upsert_spending(
    pool: &Pool<Sqlite>,
    wallet_id: &str,
    rule_type: &str,
    token_address: &str,
    chain_id: &str,
    period: &str,
    amount: &str,
    reset_at: DateTime<Utc>,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO spending_trackers (wallet_id, rule_type, token_address, chain_id, period, spent_amount, reset_at) VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(wallet_id, rule_type, token_address, chain_id, period) DO UPDATE SET spent_amount = excluded.spent_amount, reset_at = excluded.reset_at",
        wallet_id,
        rule_type,
        token_address,
        chain_id,
        period,
        amount,
        reset_at
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_spending(
    pool: &Pool<Sqlite>,
    wallet_id: &str,
    rule_type: &str,
    token_address: &str,
    chain_id: &str,
    period: &str,
) -> Result<Option<SpendingTracker>> {
    let st = sqlx::query_as::<_, SpendingTracker>(
        "SELECT wallet_id, workspace_id, rule_type, token_address, chain_id, period, spent_amount, reset_at FROM spending_trackers WHERE wallet_id = ? AND rule_type = ? AND token_address = ? AND chain_id = ? AND period = ?"
    )
    .bind(wallet_id)
    .bind(rule_type)
    .bind(token_address)
    .bind(chain_id)
    .bind(period)
    .fetch_optional(pool)
    .await?;
    Ok(st)
}
