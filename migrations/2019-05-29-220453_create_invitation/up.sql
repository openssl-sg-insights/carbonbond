CREATE TABLE invitations (
  id SERIAL PRIMARY KEY,
  code VARCHAR(32) NOT NULL,
  email VARCHAR(40) NOT NULL,
  create_time TIMESTAMP NOT NULL DEFAULT Now()
)
