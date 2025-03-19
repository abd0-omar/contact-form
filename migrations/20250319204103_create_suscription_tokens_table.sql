CREATE TABLE subscription_tokens(
   id INTEGER PRIMARY KEY,
   subscription_token TEXT NOT NULL UNIQUE,
   subscriber_id TEXT NOT NULL REFERENCES subscriptions (uuid)
);