CREATE SEQUENCE IF NOT EXISTS origin_secrets_id_seq;

CREATE TABLE IF NOT EXISTS origin_secrets (
  id bigint PRIMARY KEY DEFAULT next_id_v1('origin_secrets_id_seq'),
  origin_id bigint REFERENCES origins(id),
  owner_id bigint,
  name text,
  value text,
  created_at timestamptz DEFAULT now(),
  updated_at timestamptz DEFAULT now(),
  UNIQUE (origin_id, name)
);

CREATE OR REPLACE FUNCTION insert_origin_secret_v1 (
  os_origin_id bigint,
  os_name text,
  os_value text
) RETURNS SETOF origin_secrets AS $$
  INSERT INTO origin_secrets (origin_id, name, value)
  VALUES (os_origin_id, os_name, os_value)
  RETURNING *
$$ LANGUAGE SQL VOLATILE;

CREATE OR REPLACE FUNCTION get_origin_secret_v1 (
  os_origin_id bigint,
  os_name text
) RETURNS SETOF origin_secrets AS $$
  SELECT *
  FROM origin_secrets
  WHERE name = os_name
  AND origin_id = os_origin_id
  LIMIT 1
$$ LANGUAGE SQL STABLE;

CREATE OR REPLACE FUNCTION get_origin_secrets_for_origin_v1 (
  os_origin_id bigint
) RETURNS SETOF origin_secrets AS $$
  SELECT *
  FROM origin_secrets
  WHERE origin_id = os_origin_id
$$ LANGUAGE SQL STABLE;

CREATE OR REPLACE FUNCTION delete_origin_secret_v1 (
  os_origin_id bigint,
  os_name text
) RETURNS SETOF origin_secrets AS $$
    DELETE FROM origin_secrets WHERE name = os_name AND origin_id = os_origin_id
    RETURNING *
$$ LANGUAGE SQL VOLATILE;
