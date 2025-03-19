-- cannot start a transaction within a transaction
-- https://sqlite.org/forum/forumpost/2507664507
-- BEGIN TRANSACTION;

UPDATE subscriptions 
SET status = 'confirmed' 
WHERE status IS NULL;

CREATE TABLE subscriptions_new (
    -- Changed the id column to INTEGER PRIMARY KEY
    -- because SQLite has an implicit rowid, which improves performance.
    -- Using a UUID as the primary key would remove rowid and slow down queries.
    -- Instead, we store UUID separately as a UNIQUE column to ensure global uniqueness.
    id INTEGER PRIMARY KEY,
    uuid TEXT NOT NULL UNIQUE,  -- UUID is unique but NOT the primary key however
    -- we'll use it as a primary key
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    subscribed_at TEXT NOT NULL,
    status TEXT NOT NULL
);


INSERT INTO subscriptions_new (uuid, name, email, subscribed_at, status)
SELECT id, name, email, subscribed_at, status FROM subscriptions;

DROP TABLE subscriptions;
ALTER TABLE subscriptions_new RENAME TO subscriptions;

-- COMMIT;
