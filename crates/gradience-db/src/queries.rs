use crate::models::*;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Row, Sqlite};

// ========== Users ==========
pub async fn create_user(pool: &Pool<Sqlite>, id: &str, email: &str) -> Result<()> {
    sqlx::query!("INSERT INTO users (id, email) VALUES (?, ?)", id, email)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_user_by_email(pool: &Pool<Sqlite>, email: &str) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, email, created_at, updated_at, status FROM users WHERE email = ?",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

// ========== Workspaces ==========
pub async fn create_workspace(
    pool: &Pool<Sqlite>,
    id: &str,
    name: &str,
    owner_id: &str,
    plan: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO workspaces (id, name, owner_id, plan) VALUES (?, ?, ?, ?)",
        id,
        name,
        owner_id,
        plan
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_workspaces_by_owner(
    pool: &Pool<Sqlite>,
    owner_id: &str,
) -> Result<Vec<Workspace>> {
    let rows = sqlx::query_as::<_, Workspace>(
        "SELECT id, name, owner_id, plan, created_at FROM workspaces WHERE owner_id = ? ORDER BY created_at DESC"
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn add_workspace_member(
    pool: &Pool<Sqlite>,
    workspace_id: &str,
    user_id: &str,
    role: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO workspace_members (workspace_id, user_id, role) VALUES (?, ?, ?)",
        workspace_id,
        user_id,
        role
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_workspace_members(
    pool: &Pool<Sqlite>,
    workspace_id: &str,
) -> Result<Vec<WorkspaceMember>> {
    let rows = sqlx::query_as::<_, WorkspaceMember>(
        "SELECT workspace_id, user_id, role, invited_at FROM workspace_members WHERE workspace_id = ?"
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
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

pub async fn update_wallet_status(pool: &Pool<Sqlite>, id: &str, status: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE wallets SET status = ?, updated_at = datetime('now') WHERE id = ?",
        status,
        id
    )
    .execute(pool)
    .await?;
    Ok(())
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

pub async fn list_wallet_addresses(
    pool: &Pool<Sqlite>,
    wallet_id: &str,
) -> Result<Vec<WalletAddress>> {
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
        "SELECT id, wallet_id, name, key_hash, permissions, expires_at, last_used_at, created_at FROM api_keys WHERE key_hash = ? AND (expires_at IS NULL OR expires_at > datetime('now'))"
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

pub async fn list_api_keys_by_wallet_and_permission(
    pool: &Pool<Sqlite>,
    wallet_id: &str,
    permission: &str,
) -> Result<Vec<ApiKey>> {
    let keys = sqlx::query_as::<_, ApiKey>(
        "SELECT id, wallet_id, name, key_hash, permissions, expires_at, last_used_at, created_at FROM api_keys WHERE wallet_id = ? AND permissions = ? ORDER BY created_at DESC"
    )
    .bind(wallet_id)
    .bind(permission)
    .fetch_all(pool)
    .await?;
    Ok(keys)
}

pub async fn get_api_key_by_id(pool: &Pool<Sqlite>, id: &str) -> Result<Option<ApiKey>> {
    let key = sqlx::query_as::<_, ApiKey>(
        "SELECT id, wallet_id, name, key_hash, permissions, expires_at, last_used_at, created_at FROM api_keys WHERE id = ?"
    )
    .bind(id)
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

pub async fn list_active_policies_by_wallet(
    pool: &Pool<Sqlite>,
    wallet_id: &str,
) -> Result<Vec<Policy>> {
    let policies = sqlx::query_as::<_, Policy>(
        "SELECT id, name, wallet_id, workspace_id, rules_json, priority, status, version, created_at, updated_at FROM policies WHERE wallet_id = ? AND status = 'active' ORDER BY priority DESC"
    )
    .bind(wallet_id)
    .fetch_all(pool)
    .await?;
    Ok(policies)
}

pub async fn list_active_policies_by_workspace(
    pool: &Pool<Sqlite>,
    workspace_id: &str,
) -> Result<Vec<Policy>> {
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

pub async fn list_audit_logs_by_wallet(
    pool: &Pool<Sqlite>,
    wallet_id: &str,
    limit: i64,
) -> Result<Vec<AuditLog>> {
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

// ========== Shared Budget Trackers ==========
pub async fn upsert_shared_budget_total(
    pool: &Pool<Sqlite>,
    workspace_id: &str,
    token_address: &str,
    chain_id: &str,
    period: &str,
    total_amount: &str,
    reset_at: DateTime<Utc>,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO shared_budget_trackers (workspace_id, token_address, chain_id, period, total_amount, reset_at) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(workspace_id, token_address, chain_id, period) DO UPDATE SET total_amount = excluded.total_amount, reset_at = excluded.reset_at",
        workspace_id,
        token_address,
        chain_id,
        period,
        total_amount,
        reset_at
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_shared_budget_spending(
    pool: &Pool<Sqlite>,
    workspace_id: &str,
    token_address: &str,
    chain_id: &str,
    period: &str,
    amount: &str,
    reset_at: DateTime<Utc>,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO shared_budget_trackers (workspace_id, token_address, chain_id, period, spent_amount, reset_at) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(workspace_id, token_address, chain_id, period) DO UPDATE SET spent_amount = excluded.spent_amount, reset_at = excluded.reset_at",
        workspace_id,
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

pub async fn get_shared_budget_spending(
    pool: &Pool<Sqlite>,
    workspace_id: &str,
    token_address: &str,
    chain_id: &str,
    period: &str,
) -> Result<Option<SharedBudgetTracker>> {
    let st = sqlx::query_as::<_, SharedBudgetTracker>(
        "SELECT workspace_id, token_address, chain_id, period, spent_amount, total_amount, reset_at FROM shared_budget_trackers WHERE workspace_id = ? AND token_address = ? AND chain_id = ? AND period = ?"
    )
    .bind(workspace_id)
    .bind(token_address)
    .bind(chain_id)
    .bind(period)
    .fetch_optional(pool)
    .await?;
    Ok(st)
}

// ========== AI Balances ==========
pub async fn get_ai_balance(
    pool: &Pool<Sqlite>,
    wallet_id: &str,
    token: &str,
) -> Result<Option<AiBalance>> {
    let bal = sqlx::query_as::<_, AiBalance>(
        "SELECT wallet_id, token, balance_raw, updated_at FROM ai_balances WHERE wallet_id = ? AND token = ?"
    )
    .bind(wallet_id)
    .bind(token)
    .fetch_optional(pool)
    .await?;
    Ok(bal)
}

pub async fn upsert_ai_balance(
    pool: &Pool<Sqlite>,
    wallet_id: &str,
    token: &str,
    balance_raw: &str,
) -> Result<()> {
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
pub async fn get_model_pricing(
    pool: &Pool<Sqlite>,
    provider: &str,
    model: &str,
) -> Result<Option<ModelPricing>> {
    let pricing = sqlx::query_as::<_, ModelPricing>(
        "SELECT id, provider, model, input_per_m, output_per_m, cache_per_m, currency, effective_from, effective_to FROM model_pricing WHERE provider = ? AND model = ? AND (effective_to IS NULL OR effective_to > datetime('now')) ORDER BY effective_from DESC LIMIT 1"
    )
    .bind(provider)
    .bind(model)
    .fetch_optional(pool)
    .await?;
    Ok(pricing)
}

pub async fn get_all_model_pricing(pool: &Pool<Sqlite>) -> Result<Vec<ModelPricing>> {
    let rows = sqlx::query_as::<_, ModelPricing>(
        "SELECT id, provider, model, input_per_m, output_per_m, cache_per_m, currency, effective_from, effective_to FROM model_pricing WHERE effective_to IS NULL OR effective_to > datetime('now') ORDER BY provider, model"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
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

    // OpenAI GPT-4o
    sqlx::query(
        "INSERT OR IGNORE INTO model_pricing (id, provider, model, input_per_m, output_per_m, cache_per_m, currency, effective_from) VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))"
    )
    .bind(2i64)
    .bind("openai")
    .bind("gpt-4o")
    .bind(2500000i64) // $2.5 per 1M tokens
    .bind(10000000i64) // $10 per 1M tokens
    .bind(0i64)
    .bind("USDC")
    .execute(pool)
    .await?;

    // OpenAI GPT-4o-mini
    sqlx::query(
        "INSERT OR IGNORE INTO model_pricing (id, provider, model, input_per_m, output_per_m, cache_per_m, currency, effective_from) VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))"
    )
    .bind(3i64)
    .bind("openai")
    .bind("gpt-4o-mini")
    .bind(150000i64) // $0.15 per 1M tokens
    .bind(600000i64) // $0.6 per 1M tokens
    .bind(0i64)
    .bind("USDC")
    .execute(pool)
    .await?;
    Ok(())
}

// ========== Policy Approvals ==========
pub async fn create_policy_approval(
    pool: &Pool<Sqlite>,
    id: &str,
    policy_id: &str,
    wallet_id: &str,
    request_json: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO policy_approvals (id, policy_id, wallet_id, request_json) VALUES (?, ?, ?, ?)",
        id,
        policy_id,
        wallet_id,
        request_json
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_policy_approval(pool: &Pool<Sqlite>, id: &str) -> Result<Option<PolicyApproval>> {
    let row = sqlx::query_as::<_, PolicyApproval>(
        "SELECT id, policy_id, wallet_id, request_json, status, approved_by, approved_at, expires_at, created_at FROM policy_approvals WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn list_pending_policy_approvals(
    pool: &Pool<Sqlite>,
    wallet_id: &str,
) -> Result<Vec<PolicyApproval>> {
    let rows = sqlx::query_as::<_, PolicyApproval>(
        "SELECT id, policy_id, wallet_id, request_json, status, approved_by, approved_at, expires_at, created_at FROM policy_approvals WHERE wallet_id = ? AND status = 'pending' ORDER BY created_at DESC"
    )
    .bind(wallet_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_all_pending_policy_approvals(pool: &Pool<Sqlite>) -> Result<Vec<PolicyApproval>> {
    let rows = sqlx::query_as::<_, PolicyApproval>(
        "SELECT id, policy_id, wallet_id, request_json, status, approved_by, approved_at, expires_at, created_at FROM policy_approvals WHERE status = 'pending' ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn update_policy_approval_status(
    pool: &Pool<Sqlite>,
    id: &str,
    status: &str,
    approved_by: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE policy_approvals SET status = ?, approved_by = ?, approved_at = datetime('now') WHERE id = ?",
        status,
        approved_by,
        id
    )
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

pub async fn list_unanchored_audit_logs_for_wallet(
    pool: &Pool<Sqlite>,
    wallet_id: &str,
    limit: i64,
) -> Result<Vec<AuditLog>> {
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

// ========== Recovery Codes ==========
pub async fn create_recovery_code(
    pool: &Pool<Sqlite>,
    id: &str,
    user_id: &str,
    code: &str,
    purpose: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO recovery_codes (id, user_id, code, purpose) VALUES (?, ?, ?, ?)",
        id,
        user_id,
        code,
        purpose
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_valid_recovery_code(
    pool: &Pool<Sqlite>,
    user_id: &str,
    code: &str,
    purpose: &str,
) -> Result<Option<RecoveryCode>> {
    let row = sqlx::query_as::<_, RecoveryCode>(
        "SELECT id, user_id, code, purpose, used_at, expires_at, created_at FROM recovery_codes WHERE user_id = ? AND code = ? AND purpose = ? AND used_at IS NULL AND expires_at > datetime('now') LIMIT 1"
    )
    .bind(user_id)
    .bind(code)
    .bind(purpose)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn mark_recovery_code_used(pool: &Pool<Sqlite>, id: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE recovery_codes SET used_at = datetime('now') WHERE id = ?",
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}

// ========== OAuth Identities ==========
pub async fn create_oauth_identity(
    pool: &Pool<Sqlite>,
    id: &str,
    user_id: &str,
    provider: &str,
    provider_user_id: &str,
    email: Option<&str>,
    metadata_json: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO oauth_identities (id, user_id, provider, provider_user_id, email, metadata_json) VALUES (?, ?, ?, ?, ?, ?)",
        id,
        user_id,
        provider,
        provider_user_id,
        email,
        metadata_json
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_oauth_identity(
    pool: &Pool<Sqlite>,
    provider: &str,
    provider_user_id: &str,
) -> Result<Option<OAuthIdentity>> {
    let row = sqlx::query_as::<_, OAuthIdentity>(
        "SELECT id, user_id, provider, provider_user_id, email, metadata_json, created_at, updated_at FROM oauth_identities WHERE provider = ? AND provider_user_id = ? LIMIT 1"
    )
    .bind(provider)
    .bind(provider_user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

// ========== Payment Routes ==========
pub async fn create_payment_route(
    pool: &Pool<Sqlite>,
    id: &str,
    wallet_id: &str,
    chain_id: &str,
    token_address: &str,
    priority: i32,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO wallet_payment_routes (id, wallet_id, chain_id, token_address, priority) VALUES (?, ?, ?, ?, ?)",
        id,
        wallet_id,
        chain_id,
        token_address,
        priority
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_payment_routes_by_wallet(
    pool: &Pool<Sqlite>,
    wallet_id: &str,
) -> Result<Vec<WalletPaymentRoute>> {
    let rows = sqlx::query_as::<_, WalletPaymentRoute>(
        "SELECT id, wallet_id, chain_id, token_address, priority, created_at, updated_at FROM wallet_payment_routes WHERE wallet_id = ? ORDER BY priority ASC"
    )
    .bind(wallet_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn clear_payment_routes_by_wallet(pool: &Pool<Sqlite>, wallet_id: &str) -> Result<()> {
    sqlx::query!(
        "DELETE FROM wallet_payment_routes WHERE wallet_id = ?",
        wallet_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_payment_records_by_wallet(
    pool: &Pool<Sqlite>,
    wallet_id: &str,
) -> Result<Vec<PaymentRecord>> {
    let rows = sqlx::query_as::<_, PaymentRecord>(
        "SELECT id, wallet_id, protocol, amount, token, recipient, status, tx_hash, created_at FROM payment_records WHERE wallet_id = ? ORDER BY created_at DESC"
    )
    .bind(wallet_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ========== Email Verifications ==========
pub async fn upsert_email_verification(
    pool: &Pool<Sqlite>,
    email: &str,
    code: &str,
    expires_at: DateTime<Utc>,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO email_verifications (email, code, expires_at, attempts) VALUES (?, ?, ?, 0)\n         ON CONFLICT(email) DO UPDATE SET code = excluded.code, expires_at = excluded.expires_at, attempts = 0, created_at = datetime('now')",
        email,
        code,
        expires_at
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_email_verification(
    pool: &Pool<Sqlite>,
    email: &str,
) -> Result<Option<(String, DateTime<Utc>, i64)>> {
    let row =
        sqlx::query("SELECT code, expires_at, attempts FROM email_verifications WHERE email = ?")
            .bind(email)
            .fetch_optional(pool)
            .await?;

    Ok(row.map(|r| {
        (
            r.get::<String, _>("code"),
            r.get::<DateTime<Utc>, _>("expires_at"),
            r.get::<i64, _>("attempts"),
        )
    }))
}

pub async fn increment_email_verification_attempts(pool: &Pool<Sqlite>, email: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE email_verifications SET attempts = attempts + 1 WHERE email = ?",
        email
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_email_verification(pool: &Pool<Sqlite>, email: &str) -> Result<()> {
    sqlx::query!("DELETE FROM email_verifications WHERE email = ?", email)
        .execute(pool)
        .await?;
    Ok(())
}

// ========== Sessions ==========
pub async fn create_session(
    pool: &Pool<Sqlite>,
    token: &str,
    user_id: &str,
    username: &str,
    passphrase: Option<&str>,
    expires_at: DateTime<Utc>,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO sessions (token, user_id, username, passphrase, expires_at) VALUES (?, ?, ?, ?, ?)",
        token,
        user_id,
        username,
        passphrase,
        expires_at
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_session_by_token(
    pool: &Pool<Sqlite>,
    token: &str,
) -> Result<Option<(String, String, Option<String>)>> {
    let row = sqlx::query(
        "SELECT user_id, username, passphrase FROM sessions WHERE token = ? AND expires_at > datetime('now')"
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| {
        (
            r.get::<String, _>("user_id"),
            r.get::<String, _>("username"),
            r.get::<Option<String>, _>("passphrase"),
        )
    }))
}

pub async fn update_session_passphrase(
    pool: &Pool<Sqlite>,
    token: &str,
    passphrase: &str,
) -> Result<()> {
    sqlx::query!(
        "UPDATE sessions SET passphrase = ? WHERE token = ?",
        passphrase,
        token
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_session(pool: &Pool<Sqlite>, token: &str) -> Result<()> {
    sqlx::query!("DELETE FROM sessions WHERE token = ?", token)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_expired_sessions(pool: &Pool<Sqlite>) -> Result<u64> {
    let res = sqlx::query!("DELETE FROM sessions WHERE expires_at <= datetime('now')")
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

pub async fn delete_sessions_by_user(pool: &Pool<Sqlite>, user_id: &str) -> Result<()> {
    sqlx::query!("DELETE FROM sessions WHERE user_id = ?", user_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ========== Email Send Limits ==========
pub async fn get_email_send_limit(
    pool: &Pool<Sqlite>,
    email: &str,
) -> Result<Option<(DateTime<Utc>, i64)>> {
    let row = sqlx::query("SELECT last_sent, count_1h FROM email_send_limits WHERE email = ?")
        .bind(email)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| {
        (
            r.get::<DateTime<Utc>, _>("last_sent"),
            r.get::<i64, _>("count_1h"),
        )
    }))
}

pub async fn record_email_send(pool: &Pool<Sqlite>, email: &str) -> Result<()> {
    let now = Utc::now();
    sqlx::query!(
        "INSERT INTO email_send_limits (email, last_sent, count_1h) VALUES (?, ?, 1)
         ON CONFLICT(email) DO UPDATE SET last_sent = excluded.last_sent, count_1h = count_1h + 1",
        email,
        now
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn reset_email_send_limit(pool: &Pool<Sqlite>, email: &str) -> Result<()> {
    let now = Utc::now();
    sqlx::query!(
        "UPDATE email_send_limits SET last_sent = ?, count_1h = 1 WHERE email = ?",
        now,
        email
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_sessions_by_user(
    pool: &Pool<Sqlite>,
    user_id: &str,
) -> Result<Vec<(String, String, DateTime<Utc>, DateTime<Utc>)>> {
    let rows = sqlx::query(
        "SELECT token, username, created_at, expires_at FROM sessions WHERE user_id = ? AND expires_at > datetime('now') ORDER BY created_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            (
                r.get::<String, _>("token"),
                r.get::<String, _>("username"),
                r.get::<DateTime<Utc>, _>("created_at"),
                r.get::<DateTime<Utc>, _>("expires_at"),
            )
        })
        .collect())
}

pub async fn delete_session_by_token(pool: &Pool<Sqlite>, token: &str) -> Result<u64> {
    let res = sqlx::query!("DELETE FROM sessions WHERE token = ?", token)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}
