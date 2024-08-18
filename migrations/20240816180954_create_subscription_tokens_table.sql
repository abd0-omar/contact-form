-- Add migration script here
CREATE TABLE subscription_tokens(
    subscription_token TEXT NOT NULL PRIMARY KEY,
    subscriber_id TEXT NOT NULL,
    FOREIGN KEY (subscriber_id) REFERENCES subscriptions(id)
);
