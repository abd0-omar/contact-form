-- Add migration script here
-- Create Subscriptions Table
CREATE TABLE subscriptions(
    id uuid NOT NULL,
    PRIMARY KEY(id),
    -- UNIQUE, in particular, introduces an additional B-tree index on our email column: the index has to
    -- be updated on every INSERT/UPDATE/DELETE query and it takes space on disk.
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    -- timestamp with a timezone, hence the 'tz' at the end
    subscribed_at timestamptz NOT NULL
);