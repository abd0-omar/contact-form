-- Add up migration script here
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    username text NOT NULL UNIQUE,
    password text NOT NULL
);

-- similar but has more checks, maybe for later
-- CREATE TABLE IF NOT EXISTS users (
--     id SERIAL PRIMARY KEY,
--     username TEXT NOT NULL UNIQUE CHECK (char_length(username) >= 3),
--     password TEXT NOT NULL CHECK (char_length(password) >= 8),
--     email TEXT UNIQUE,
--     created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
--     updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
-- );
--
