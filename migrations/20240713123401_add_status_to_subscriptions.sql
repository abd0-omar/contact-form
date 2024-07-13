-- Add migration script here
-- make it NULL (optional) for now to preserve zero-down-time
ALTER TABLE subscriptions ADD COLUMN status TEXT NULL;
