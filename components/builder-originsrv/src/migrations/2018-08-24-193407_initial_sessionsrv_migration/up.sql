CREATE SEQUENCE IF NOT EXISTS accounts_id_seq; 
CREATE SEQUENCE IF NOT EXISTS account_tokens_id_seq;

CREATE TABLE IF NOT EXISTS accounts (
    id bigint DEFAULT next_id_v1('accounts_id_seq') PRIMARY KEY NOT NULL,
    name text UNIQUE,
    email text,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now()
);

CREATE TABLE IF NOT EXISTS account_tokens (
    id bigint DEFAULT next_id_v1('account_tokens_id_seq') PRIMARY KEY NOT NULL,
    account_id bigint,
    token text UNIQUE,
    created_at timestamp with time zone DEFAULT now()
);

CREATE OR REPLACE FUNCTION get_account_by_id_v1(account_id bigint) RETURNS SETOF accounts
    LANGUAGE plpgsql STABLE
    AS $$
    BEGIN
      RETURN QUERY SELECT * FROM accounts WHERE id = account_id;
      RETURN;
    END
$$;

CREATE OR REPLACE FUNCTION get_account_by_name_v1(account_name text) RETURNS SETOF accounts
    LANGUAGE plpgsql STABLE
    AS $$
    BEGIN
      RETURN QUERY SELECT * FROM accounts WHERE name = account_name;
      RETURN;
    END
$$;

CREATE OR REPLACE FUNCTION get_account_token_with_id_v1(p_id bigint) RETURNS SETOF account_tokens
    LANGUAGE sql STABLE
    AS $$
    SELECT * FROM account_tokens WHERE id = p_id;
$$;

CREATE OR REPLACE FUNCTION get_account_tokens_v1(p_account_id bigint) RETURNS SETOF account_tokens
    LANGUAGE sql STABLE
    AS $$
    SELECT * FROM account_tokens WHERE account_id = p_account_id;
$$;

CREATE OR REPLACE FUNCTION insert_account_token_v1(p_account_id bigint, p_token text) RETURNS SETOF account_tokens
    LANGUAGE sql
    AS $$
    DELETE FROM account_tokens WHERE account_id = p_account_id;
    INSERT INTO account_tokens (account_id, token)
    VALUES (p_account_id, p_token)
    RETURNING *;
$$;

CREATE OR REPLACE FUNCTION revoke_account_token_v1(p_id bigint) RETURNS void
    LANGUAGE sql
    AS $$
    DELETE FROM account_tokens WHERE id = p_id;
$$;

CREATE OR REPLACE FUNCTION select_or_insert_account_v1(account_name text, account_email text) RETURNS SETOF accounts
    LANGUAGE plpgsql
    AS $$
    DECLARE
      existing_account accounts%rowtype;
    BEGIN
      SELECT * INTO existing_account FROM accounts WHERE name = account_name LIMIT 1;
      IF FOUND THEN
          RETURN NEXT existing_account;
      ELSE
          RETURN QUERY INSERT INTO accounts (name, email) VALUES (account_name, account_email) ON CONFLICT DO NOTHING RETURNING *;
      END IF;
      RETURN;
    END
$$;

CREATE OR REPLACE FUNCTION update_account_v1(op_id bigint, op_email text) RETURNS void
    LANGUAGE sql
    AS $$
    UPDATE accounts SET email = op_email WHERE id = op_id;
$$;
