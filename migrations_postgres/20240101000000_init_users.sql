-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY NOT NULL,
    subject TEXT NOT NULL,
    email TEXT NOT NULL,
    password_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_subject ON users(subject);
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email ON users(email);
