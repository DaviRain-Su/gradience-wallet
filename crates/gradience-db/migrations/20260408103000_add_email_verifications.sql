CREATE TABLE email_verifications (
    email TEXT PRIMARY KEY,
    code TEXT NOT NULL,
    expires_at DATETIME NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_email_verifications_expires ON email_verifications(expires_at);
