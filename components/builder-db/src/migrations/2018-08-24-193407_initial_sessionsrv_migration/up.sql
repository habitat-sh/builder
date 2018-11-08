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
