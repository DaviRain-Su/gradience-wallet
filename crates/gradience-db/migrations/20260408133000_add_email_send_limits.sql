CREATE TABLE email_send_limits (
    email TEXT PRIMARY KEY,
    last_sent DATETIME NOT NULL,
    count_1h INTEGER NOT NULL DEFAULT 1
);
