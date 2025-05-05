CREATE SEQUENCE IF NOT EXISTS license_keys_id_seq;

CREATE TABLE IF NOT EXISTS license_keys (
    id bigint DEFAULT next_id_v1('license_keys_id_seq') PRIMARY KEY NOT NULL,
    account_id bigint UNIQUE NOT NULL,
    license_key text NOT NULL,
    expiration_date date NOT NULL,
    created_at timestamp with time zone DEFAULT now()
);
