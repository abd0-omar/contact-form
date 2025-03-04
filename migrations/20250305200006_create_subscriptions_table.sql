-- Add migration script here
CREATE TABLE subscriptions (
    -- uuid
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    -- timestamp with time zone
    subscribed_at TEXT NOT NULL
);
