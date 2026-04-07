CREATE TABLE users (
    id          TEXT PRIMARY KEY,
    email       TEXT UNIQUE NOT NULL,
    created_at  DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at  DATETIME NOT NULL DEFAULT (datetime('now')),
    status      TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'suspended'))
);

CREATE TABLE passkey_credentials (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id   BLOB UNIQUE NOT NULL,
    credential_pk   BLOB NOT NULL,
    counter         INTEGER NOT NULL DEFAULT 0,
    transports      TEXT NOT NULL DEFAULT '[]',
    device_name     TEXT,
    last_used_at    DATETIME,
    created_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_passkey_user ON passkey_credentials(user_id);

CREATE TABLE workspaces (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    owner_id        TEXT NOT NULL REFERENCES users(id),
    plan            TEXT NOT NULL DEFAULT 'free'
        CHECK (plan IN ('free', 'pro', 'team', 'enterprise')),
    created_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_workspaces_owner ON workspaces(owner_id);

CREATE TABLE workspace_members (
    workspace_id    TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            TEXT NOT NULL DEFAULT 'member'
        CHECK (role IN ('owner', 'admin', 'member', 'viewer')),
    invited_at      DATETIME NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (workspace_id, user_id)
);

CREATE TABLE wallets (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    owner_id        TEXT NOT NULL REFERENCES users(id),
    workspace_id    TEXT REFERENCES workspaces(id),
    status          TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'suspended', 'revoked')),
    created_at      DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_wallets_owner ON wallets(owner_id);
CREATE INDEX idx_wallets_workspace ON wallets(workspace_id);

CREATE TABLE wallet_addresses (
    id              TEXT PRIMARY KEY,
    wallet_id       TEXT NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    chain_id        TEXT NOT NULL,
    address         TEXT NOT NULL,
    derivation_path TEXT NOT NULL,
    UNIQUE(wallet_id, chain_id)
);

CREATE INDEX idx_wallet_addresses_wallet ON wallet_addresses(wallet_id);

CREATE TABLE api_keys (
    id              TEXT PRIMARY KEY,
    wallet_id       TEXT NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    key_hash        BLOB NOT NULL,
    permissions     TEXT NOT NULL DEFAULT '["sign", "read"]',
    expires_at      DATETIME,
    last_used_at    DATETIME,
    created_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_api_keys_wallet ON api_keys(wallet_id);
CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);

CREATE TABLE policies (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    wallet_id       TEXT REFERENCES wallets(id) ON DELETE CASCADE,
    workspace_id    TEXT REFERENCES workspaces(id) ON DELETE CASCADE,
    rules_json      TEXT NOT NULL,
    priority        INTEGER NOT NULL DEFAULT 1
        CHECK (priority IN (0, 1)),
    status          TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'paused', 'deleted')),
    version         INTEGER NOT NULL DEFAULT 1,
    created_at      DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now')),
    CONSTRAINT chk_policy_scope CHECK (
        wallet_id IS NOT NULL OR workspace_id IS NOT NULL
    )
);

CREATE INDEX idx_policies_wallet ON policies(wallet_id, status);
CREATE INDEX idx_policies_workspace ON policies(workspace_id, status);

CREATE TABLE spending_trackers (
    wallet_id       TEXT NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    workspace_id    TEXT,
    rule_type       TEXT NOT NULL
        CHECK (rule_type IN ('daily', 'monthly')),
    token_address   TEXT NOT NULL,
    chain_id        TEXT NOT NULL,
    period          TEXT NOT NULL,
    spent_amount    TEXT NOT NULL DEFAULT '0',
    reset_at        DATETIME NOT NULL,
    PRIMARY KEY (wallet_id, rule_type, token_address, chain_id, period)
);

CREATE INDEX idx_spending_workspace ON spending_trackers(workspace_id, period);

CREATE TABLE policy_approvals (
    id              TEXT PRIMARY KEY,
    policy_id       TEXT NOT NULL REFERENCES policies(id),
    wallet_id       TEXT NOT NULL REFERENCES wallets(id),
    request_json    TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'rejected', 'expired')),
    approved_by     TEXT REFERENCES users(id),
    approved_at     DATETIME,
    expires_at      DATETIME NOT NULL
        DEFAULT (datetime('now', '+30 minutes')),
    created_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_approvals_wallet ON policy_approvals(wallet_id, status);
CREATE INDEX idx_approvals_pending ON policy_approvals(status, expires_at)
    WHERE status = 'pending';

CREATE TABLE audit_logs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id       TEXT NOT NULL REFERENCES wallets(id),
    api_key_id      TEXT REFERENCES api_keys(id),
    action          TEXT NOT NULL,
    context_json    TEXT NOT NULL,
    intent_json     TEXT,
    decision        TEXT NOT NULL
        CHECK (decision IN ('allowed', 'denied', 'warned')),
    decision_reason TEXT,
    dynamic_factors TEXT,
    tx_hash         TEXT,
    anchor_tx_hash  TEXT,
    anchor_root     TEXT,
    prev_hash       TEXT NOT NULL,
    current_hash    TEXT NOT NULL,
    created_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_audit_wallet_time ON audit_logs(wallet_id, created_at DESC);
CREATE INDEX idx_audit_anchor ON audit_logs(anchor_root)
    WHERE anchor_root IS NOT NULL;

CREATE TABLE anchor_batches (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    root            TEXT NOT NULL,
    prev_root       TEXT,
    log_start_index INTEGER NOT NULL,
    log_end_index   INTEGER NOT NULL,
    leaf_count      INTEGER NOT NULL,
    tx_hash         TEXT NOT NULL,
    block_number    INTEGER,
    anchored_at     DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_anchor_root ON anchor_batches(root);

CREATE TABLE payment_records (
    id              TEXT PRIMARY KEY,
    wallet_id       TEXT NOT NULL REFERENCES wallets(id),
    protocol        TEXT NOT NULL
        CHECK (protocol IN ('x402', 'mpp', 'hsp')),
    amount          TEXT NOT NULL,
    token           TEXT NOT NULL,
    recipient       TEXT NOT NULL,
    status          TEXT NOT NULL
        CHECK (status IN ('pending', 'completed', 'failed')),
    tx_hash         TEXT,
    created_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE ai_balances (
    wallet_id       TEXT NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    token           TEXT NOT NULL DEFAULT 'USDC',
    balance_raw     TEXT NOT NULL DEFAULT '0',
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (wallet_id, token)
);

CREATE TABLE llm_call_logs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id       TEXT NOT NULL REFERENCES wallets(id),
    api_key_id      TEXT REFERENCES api_keys(id),
    provider        TEXT NOT NULL,
    model           TEXT NOT NULL,
    input_tokens    INTEGER NOT NULL,
    output_tokens   INTEGER NOT NULL,
    cached_tokens   INTEGER DEFAULT 0,
    cost_raw        TEXT NOT NULL,
    duration_ms     INTEGER,
    status          TEXT NOT NULL DEFAULT 'success'
        CHECK (status IN ('success', 'denied', 'budget_exceeded')),
    created_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_llm_wallet ON llm_call_logs(wallet_id, created_at DESC);

CREATE TABLE model_pricing (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    provider        TEXT NOT NULL,
    model           TEXT NOT NULL,
    input_per_m     INTEGER NOT NULL,
    output_per_m    INTEGER NOT NULL,
    cache_per_m     INTEGER NOT NULL DEFAULT 0,
    currency        TEXT NOT NULL DEFAULT 'USDC',
    effective_from  DATETIME NOT NULL DEFAULT (datetime('now')),
    effective_to    DATETIME,
    UNIQUE(provider, model, effective_from)
);
