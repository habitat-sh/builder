CREATE SEQUENCE IF NOT EXISTS origin_public_encryption_key_id_seq;

CREATE TABLE IF NOT EXISTS origin_public_encryption_keys (
  id bigint PRIMARY KEY DEFAULT next_id_v1('origin_public_key_id_seq'),
  origin_id bigint REFERENCES origins(id),
  owner_id bigint,
  name text,
  revision text,
  full_name text UNIQUE,
  body bytea,
  created_at timestamptz DEFAULT now(),
  updated_at timestamptz DEFAULT now()
);

CREATE OR REPLACE FUNCTION insert_origin_public_encryption_key_v1 (
  opek_origin_id bigint,
  opek_owner_id bigint,
  opek_name text,
  opek_revision text,
  opek_full_name text,
  opek_body bytea
) RETURNS SETOF origin_public_encryption_keys AS $$
    BEGIN
      RETURN QUERY INSERT INTO origin_public_encryption_keys (origin_id, owner_id, name, revision, full_name, body)
          VALUES (opek_origin_id, opek_owner_id, opek_name, opek_revision, opek_full_name, opek_body)
          RETURNING *;
      RETURN;
    END
$$ LANGUAGE plpgsql VOLATILE;

CREATE OR REPLACE FUNCTION get_origin_public_encryption_key_v1 (
  opek_name text,
  opek_revision text
) RETURNS SETOF origin_public_encryption_keys AS $$
  BEGIN
    RETURN QUERY SELECT * FROM origin_public_encryption_keys WHERE name = opek_name and revision = opek_revision
      ORDER BY revision DESC
      LIMIT 1;
    RETURN;
  END
$$ LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION get_origin_public_encryption_key_latest_v1 (
  opek_name text
) RETURNS SETOF origin_public_encryption_keys AS $$
  BEGIN
    RETURN QUERY SELECT * FROM origin_public_encryption_keys WHERE name = opek_name
      ORDER BY revision DESC
      LIMIT 1;
    RETURN;
  END
$$ LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION get_origin_public_encryption_keys_for_origin_v1 (
  opek_origin_id bigint
) RETURNS SETOF origin_public_encryption_keys AS $$
  BEGIN
      RETURN QUERY SELECT * FROM origin_public_encryption_keys WHERE origin_id = opek_origin_id
        ORDER BY revision DESC;
      RETURN;
  END
$$ LANGUAGE plpgsql STABLE;

CREATE SEQUENCE IF NOT EXISTS origin_private_encryption_key_id_seq;

CREATE TABLE IF NOT EXISTS origin_private_encryption_keys (
  id bigint PRIMARY KEY DEFAULT next_id_v1('origin_private_encryption_key_id_seq'),
  origin_id bigint REFERENCES origins(id),
  owner_id bigint,
  name text,
  revision text,
  full_name text UNIQUE,
  body bytea,
  created_at timestamptz DEFAULT now(),
  updated_at timestamptz
);

CREATE OR REPLACE FUNCTION insert_origin_private_encryption_key_v1 (
  opek_origin_id bigint,
  opek_owner_id bigint,
  opek_name text,
  opek_revision text,
  opek_full_name text,
  opek_body bytea
) RETURNS SETOF origin_private_encryption_keys AS $$
  BEGIN
    RETURN QUERY INSERT INTO origin_private_encryption_keys (origin_id, owner_id, name, revision, full_name, body)
          VALUES (opek_origin_id, opek_owner_id, opek_name, opek_revision, opek_full_name, opek_body)
          RETURNING *;
    RETURN;
  END
$$ LANGUAGE plpgsql VOLATILE;

CREATE OR REPLACE FUNCTION get_origin_private_encryption_key_v1 (
  opek_name text
) RETURNS SETOF origin_private_encryption_keys AS $$
  BEGIN
    RETURN QUERY SELECT * FROM origin_private_encryption_keys WHERE name = opek_name
      ORDER BY full_name DESC
      LIMIT 1;
    RETURN;
  END
  $$ LANGUAGE plpgsql STABLE;

CREATE OR REPLACE VIEW origins_with_private_encryption_key_full_name_v1 AS
  SELECT origins.id, origins.name, origins.owner_id,
          origin_private_encryption_keys.full_name AS private_key_name,
          origins.default_package_visibility
    FROM origins
    LEFT OUTER JOIN origin_private_encryption_keys ON (origins.id = origin_private_encryption_keys.origin_id)
    ORDER BY origins.id, origin_private_encryption_keys.full_name DESC;