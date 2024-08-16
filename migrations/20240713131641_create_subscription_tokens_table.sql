-- -- Add migration script here
-- CREATE TABLE subscription_tokens(
--     subscription_token TEXT NOT NULL,
--     subscriber_id uuid NOT NULL
--         REFERENCES subscriptions (id),
--     PRIMARY KEY (subscription_token)
-- );

-- Create Subscription Tokens Table
CREATE TABLE subscription_tokens(
    subscription_token TEXT NOT NULL PRIMARY KEY,
    subscriber_id TEXT NOT NULL,
    FOREIGN KEY (subscriber_id) REFERENCES subscriptions(id)
);

