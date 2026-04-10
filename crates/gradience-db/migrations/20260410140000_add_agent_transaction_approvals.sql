CREATE TABLE agent_transaction_approvals (
    id              TEXT PRIMARY KEY,
    wallet_id       TEXT NOT NULL REFERENCES wallets(id),
    session_id      TEXT,
    request_json    TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',
    approved_by     TEXT,
    approved_at     DATETIME,
    expires_at      DATETIME NOT NULL,
    created_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_agent_tx_approvals_wallet ON agent_transaction_approvals(wallet_id, status);
CREATE INDEX idx_agent_tx_approvals_expires ON agent_transaction_approvals(expires_at);
