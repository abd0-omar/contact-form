-- Add migration script here
CREATE TABLE subscriptions(
    id TEXT NOT NULL PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    subscribed_at TEXT NOT NULL,
    status TEXT NOT NULL
);