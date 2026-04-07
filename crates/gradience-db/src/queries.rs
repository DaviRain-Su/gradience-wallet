use crate::models::*;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Row, Sqlite};

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

pub async fn list_api_keys_by_wallet(pool: &Pool<Sqlite>, wallet_id: &str) -> Result<Vec<ApiKey>> {
    let keys = sqlx::query_as::<_, ApiKey>(
        "SELECT id, wallet_id, name, key_hash, permissions, expires_at, last_used_at, created_at FROM api_keys WHERE wallet_id = ? ORDER BY created_at DESC"
    )
    .bind(wallet_id)
    .fetch_all(pool)
    .await?;
    Ok(keys)
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
        "SELECT id, wallet_id, api_key_id, action, context_json, intent_json, decision, decision_reason, dynamic_factors, tx_hash, anchor_tx_hash, anchor_root, anchor_leaf_index, prev_hash, current_hash, created_at FROM audit_logs WHERE wallet_id = ? ORDER BY created_at DESC LIMIT ?"
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

// ========== AI Balances ==========
pub async fn get_ai_balance(pool: &Pool<Sqlite>, wallet_id: &str, token: &str) -> Result<Option<AiBalance>> {
    let bal = sqlx::query_as::<_, AiBalance>(
        "SELECT wallet_id, token, balance_raw, updated_at FROM ai_balances WHERE wallet_id = ? AND token = ?"
    )
    .bind(wallet_id)
    .bind(token)
    .fetch_optional(pool)
    .await?;
    Ok(bal)
}

pub async fn upsert_ai_balance(pool: &Pool<Sqlite>, wallet_id: &str, token: &str, balance_raw: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO ai_balances (wallet_id, token, balance_raw, updated_at) VALUES (?, ?, ?, datetime('now')) ON CONFLICT(wallet_id, token) DO UPDATE SET balance_raw = excluded.balance_raw, updated_at = excluded.updated_at"
    )
    .bind(wallet_id)
    .bind(token)
    .bind(balance_raw)
    .execute(pool)
    .await?;
    Ok(())
}

// ========== LLM Call Logs ==========
pub async fn insert_llm_call_log(
    pool: &Pool<Sqlite>,
    wallet_id: &str,
    api_key_id: Option<&str>,
    provider: &str,
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    cached_tokens: Option<i64>,
    cost_raw: &str,
    duration_ms: i32,
    status: &str,
) -> Result<i64> {
    let row = sqlx::query(
        "INSERT INTO llm_call_logs (wallet_id, api_key_id, provider, model, input_tokens, output_tokens, cached_tokens, cost_raw, duration_ms, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id"
    )
    .bind(wallet_id)
    .bind(api_key_id)
    .bind(provider)
    .bind(model)
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(cached_tokens)
    .bind(cost_raw)
    .bind(duration_ms)
    .bind(status)
    .fetch_one(pool)
    .await?;
    Ok(row.get("id"))
}

// ========== Model Pricing ==========
pub async fn get_model_pricing(pool: &Pool<Sqlite>, provider: &str, model: &str) -> Result<Option<ModelPricing>> {
    let pricing = sqlx::query_as::<_, ModelPricing>(
        "SELECT id, provider, model, input_per_m, output_per_m, cache_per_m, currency, effective_from, effective_to FROM model_pricing WHERE provider = ? AND model = ? AND (effective_to IS NULL OR effective_to > datetime('now')) ORDER BY effective_from DESC LIMIT 1"
    )
    .bind(provider)
    .bind(model)
    .fetch_optional(pool)
    .await?;
    Ok(pricing)
}

pub async fn seed_model_pricing(pool: &Pool<Sqlite>) -> Result<()> {
    // Anthropic Claude 3.5 Sonnet mock pricing
    sqlx::query(
        "INSERT OR IGNORE INTO model_pricing (id, provider, model, input_per_m, output_per_m, cache_per_m, currency, effective_from) VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))"
    )
    .bind(1i64)
    .bind("anthropic")
    .bind("claude-3-5-sonnet")
    .bind(3000000i64) // $3 per 1M tokens
    .bind(15000000i64) // $15 per 1M tokens
    .bind(375000i64) // $0.375 per 1M cached tokens
    .bind("USDC")
    .execute(pool)
    .await?;
    Ok(())
}

// ========== Anchor ==========
pub async fn list_unanchored_logs(pool: &Pool<Sqlite>, limit: i64) -> Result<Vec<AuditLog>> {
    let logs = sqlx::query_as::<_, AuditLog>(
        "SELECT id, wallet_id, api_key_id, action, context_json, intent_json, decision, decision_reason, dynamic_factors, tx_hash, anchor_tx_hash, anchor_root, anchor_leaf_index, prev_hash, current_hash, created_at FROM audit_logs WHERE anchor_tx_hash IS NULL ORDER BY id LIMIT ?"
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(logs)
}

pub async fn list_unanchored_audit_logs_for_wallet(pool: &Pool<Sqlite>, wallet_id: &str, limit: i64) -> Result<Vec<AuditLog>> {
    let logs = sqlx::query_as::<_, AuditLog>(
        "SELECT id, wallet_id, api_key_id, action, context_json, intent_json, decision, decision_reason, dynamic_factors, tx_hash, anchor_tx_hash, anchor_root, anchor_leaf_index, prev_hash, current_hash, created_at FROM audit_logs WHERE wallet_id = ? AND anchor_tx_hash IS NULL ORDER BY id LIMIT ?"
    )
    .bind(wallet_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(logs)
}

pub async fn mark_logs_anchored(
    pool: &Pool<Sqlite>,
    ids: &[i64],
    root: &str,
    tx_hash: &str,
) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    // SQLite does not support array parameters; use repeated OR for small batches
    let placeholders: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
    let sql = format!(
        "UPDATE audit_logs SET anchor_root = ?, anchor_tx_hash = ? WHERE id IN ({})",
        placeholders.join(",")
    );
    sqlx::query(&sql)
        .bind(root)
        .bind(tx_hash)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_anchor_batch(
    pool: &Pool<Sqlite>,
    root: &str,
    prev_root: Option<&str>,
    log_start_index: i64,
    log_end_index: i64,
    leaf_count: i32,
    tx_hash: &str,
    block_number: Option<i64>,
) -> Result<i64> {
    let row = sqlx::query(
        "INSERT INTO anchor_batches (root, prev_root, log_start_index, log_end_index, leaf_count, tx_hash, block_number) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id"
    )
    .bind(root)
    .bind(prev_root)
    .bind(log_start_index)
    .bind(log_end_index)
    .bind(leaf_count)
    .bind(tx_hash)
    .bind(block_number)
    .fetch_one(pool)
    .await?;
    Ok(row.get("id"))
}

pub async fn get_latest_anchor_batch(pool: &Pool<Sqlite>) -> Result<Option<AnchorBatch>> {
    let batch = sqlx::query_as::<_, AnchorBatch>(
        "SELECT id, root, prev_root, log_start_index, log_end_index, leaf_count, tx_hash, block_number, anchored_at FROM anchor_batches ORDER BY anchored_at DESC LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;
    Ok(batch)
}
