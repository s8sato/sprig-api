CREATE TABLE tokens (
  id UUID PRIMARY KEY,
  owner INT NOT NULL REFERENCES users ON DELETE CASCADE,
  expires_at TIMESTAMP WITH TIME ZONE NOT NULL
);
