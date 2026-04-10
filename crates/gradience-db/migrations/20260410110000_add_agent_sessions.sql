CREATE TABLE agent_sessions (
    id              TEXT PRIMARY KEY,
    wallet_id       TEXT NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    session_type    TEXT NOT NULL,
    agent_key_hash  TEXT,
    status          TEXT NOT NULL DEFAULT 'active',
    expires_at      DATETIME NOT NULL,
    created_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE agent_session_limits (
    session_id      TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    limit_type      TEXT NOT NULL,
    token           TEXT NOT NULL,
    amount_raw      TEXT NOT NULL,
    PRIMARY KEY (session_id, limit_type, token)
);

CREATE TABLE agent_session_usage (
    session_id      TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    token           TEXT NOT NULL,
    usage_date      DATE NOT NULL,
    spent_raw       TEXT NOT NULL DEFAULT '0',
    PRIMARY KEY (session_id, token, usage_date)
);

CREATE INDEX idx_agent_sessions_wallet ON agent_sessions(wallet_id, status);
CREATE INDEX idx_agent_sessions_expires ON agent_sessions(expires_at);
