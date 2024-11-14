-- Add migration script here
CREATE TABLE seen_posts (
    id SERIAL PRIMARY KEY,
    link VARCHAR NOT NULL UNIQUE
);
