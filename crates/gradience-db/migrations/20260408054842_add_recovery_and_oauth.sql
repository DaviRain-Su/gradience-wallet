-- Recovery codes for passkey reset / email verification
CREATE TABLE recovery_codes (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code        TEXT NOT NULL,
    purpose     TEXT NOT NULL DEFAULT 'passkey_recovery'
        CHECK (purpose IN ('passkey_recovery', 'email_verify', 'high_value_otp')),
    used_at     DATETIME,
    expires_at  DATETIME NOT NULL
        DEFAULT (datetime('now', '+30 minutes')),
    created_at  DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_recovery_codes_user ON recovery_codes(user_id, purpose, expires_at);
CREATE INDEX idx_recovery_codes_code ON recovery_codes(code, purpose)
    WHERE used_at IS NULL;

-- OAuth identities (Google, GitHub, etc.)
CREATE TABLE oauth_identities (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider        TEXT NOT NULL
        CHECK (provider IN ('google', 'github', 'twitter')),
    provider_user_id  TEXT NOT NULL,
    email             TEXT,
    metadata_json     TEXT,
    created_at        DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at        DATETIME NOT NULL DEFAULT (datetime('now')),
    UNIQUE(provider, provider_user_id)
);

CREATE INDEX idx_oauth_user ON oauth_identities(user_id, provider);
