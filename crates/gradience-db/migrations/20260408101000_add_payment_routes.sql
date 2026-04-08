CREATE TABLE wallet_payment_routes (
    id TEXT PRIMARY KEY,
    wallet_id TEXT NOT NULL,
    chain_id TEXT NOT NULL,
    token_address TEXT NOT NULL DEFAULT '',
    priority INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (wallet_id) REFERENCES wallets(id) ON DELETE CASCADE
);

CREATE INDEX idx_wallet_payment_routes_wallet ON wallet_payment_routes(wallet_id);
