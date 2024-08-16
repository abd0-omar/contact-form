-- Add migration script here
-- We wrap the whole migration in a transaction to make sure
-- it succeeds or fails atomically.
BEGIN;
    -- Backfill `status` for historical entries
    UPDATE subscriptions
        SET status = 'confirmed'
        WHERE status IS NULL;
    -- make `status` mandatory
    ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;


-- We wrap the whole migration in a transaction to make sure
-- it succeeds or fails atomically.
BEGIN;
    -- Backfill `status` for historical entries
    UPDATE subscriptions
        SET status = 'confirmed'
        WHERE status IS NULL;

    -- Make `status` mandatory (not nullable)
    -- had to make another temp table to update the columns, as sqlite doesn't support that
    -- also have to turn off foreign key constraints temporarly
    PRAGMA foreign_keys=off;
    CREATE TABLE subscriptions_new (
        id TEXT NOT NULL PRIMARY KEY,
        email TEXT NOT NULL UNIQUE,
        name TEXT NOT NULL,
        subscribed_at TEXT NOT NULL,
        status TEXT NOT NULL
    );

    INSERT INTO subscriptions_new (id, email, name, subscribed_at, status)
    SELECT id, email, name, subscribed_at, status FROM subscriptions;

    DROP TABLE subscriptions;
    ALTER TABLE subscriptions_new RENAME TO subscriptions;
    PRAGMA foreign_keys=on;
COMMIT;
