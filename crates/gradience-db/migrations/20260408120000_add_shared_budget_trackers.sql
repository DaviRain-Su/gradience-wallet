CREATE TABLE shared_budget_trackers (
    workspace_id    TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    token_address   TEXT NOT NULL,
    chain_id        TEXT NOT NULL,
    period          TEXT NOT NULL,
    spent_amount    TEXT NOT NULL DEFAULT '0',
    reset_at        DATETIME NOT NULL,
    PRIMARY KEY (workspace_id, token_address, chain_id, period)
);

CREATE INDEX idx_shared_budget_workspace ON shared_budget_trackers(workspace_id, period);
