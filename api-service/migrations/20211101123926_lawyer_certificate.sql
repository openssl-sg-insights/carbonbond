-- Add migration script here

ALTER TABLE signup_tokens
ADD name text NOT NULL
DEFAULT ('');

ALTER TABLE signup_tokens
ADD gender text NOT NULL
DEFAULT ('other');

ALTER TABLE signup_tokens
ADD birth date NOT NULL;

ALTER TABLE signup_tokens
ADD certificate_image bytea NOT NULL
