CREATE SEQUENCE IF NOT EXISTS account_tokens_id_seq;

CREATE TABLE IF NOT EXISTS account_tokens (
  id bigint PRIMARY KEY DEFAULT next_id_v1('account_tokens_id_seq'),
  account_id bigint,
  token text UNIQUE,
  created_at timestamptz DEFAULT now()
);

CREATE OR REPLACE FUNCTION insert_account_token_v1 (
  p_account_id bigint,
  p_token text
) RETURNS SETOF account_tokens AS $$
    DELETE FROM account_tokens WHERE account_id = p_account_id;
    INSERT INTO account_tokens (account_id, token)
    VALUES (p_account_id, p_token)
    RETURNING *;
$$ LANGUAGE SQL VOLATILE;

CREATE OR REPLACE FUNCTION get_account_tokens_v1 (
  p_account_id bigint
) RETURNS SETOF account_tokens AS $$
    SELECT * FROM account_tokens WHERE account_id = p_account_id;
$$ LANGUAGE SQL STABLE;

CREATE OR REPLACE FUNCTION revoke_account_token_v1 (
  p_id bigint
) RETURNS void AS $$
    DELETE FROM account_tokens WHERE id = p_id;
$$ LANGUAGE SQL VOLATILE;
