-- Add up migration script here
CREATE TABLE IF NOT EXISTS sessions (
    session_token BYTEA PRIMARY KEY,
    user_id INTEGER REFERENCES users (id) ON DELETE CASCADE
);

-- similar but has more checks, maybe for later
-- CREATE TABLE IF NOT EXISTS sessions (
--     session_token BYTEA PRIMARY KEY,
--     user_id INTEGER REFERENCES users (id) ON DELETE CASCADE,
--     created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
--     expires_at TIMESTAMP NOT NULL
-- );
