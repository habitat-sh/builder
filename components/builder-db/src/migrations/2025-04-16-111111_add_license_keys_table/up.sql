CREATE SEQUENCE IF NOT EXISTS license_keys_id_seq;

CREATE TABLE IF NOT EXISTS license_keys (
    id bigint DEFAULT next_id_v1('license_keys_id_seq') PRIMARY KEY NOT NULL,
    account_id bigint NOT NULL,
    license_key text UNIQUE NOT NULL,
    expiration_date text NOT NULL,
    created_at timestamp with time zone DEFAULT now()
);
