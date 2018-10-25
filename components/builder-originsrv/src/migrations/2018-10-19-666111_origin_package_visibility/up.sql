DROP VIEW origins_with_private_encryption_key_full_name_v1;
DROP VIEW origins_with_secret_key_full_name_v2;

ALTER TABLE origins ALTER COLUMN default_package_visibility DROP DEFAULT;
ALTER TABLE origins ALTER COLUMN default_package_visibility SET DATA TYPE origin_package_visibility
    USING default_package_visibility :: origin_package_visibility;
ALTER TABLE origins ALTER COLUMN default_package_visibility SET DEFAULT 'public'::origin_package_visibility;

CREATE OR REPLACE VIEW origins_with_private_encryption_key_full_name_v1 AS
 SELECT origins.id,
    origins.name,
    origins.owner_id,
    origin_private_encryption_keys.full_name AS private_key_name,
    origins.default_package_visibility
   FROM (origins
     LEFT JOIN origin_private_encryption_keys ON ((origins.id = origin_private_encryption_keys.origin_id)))
  ORDER BY origins.id, origin_private_encryption_keys.full_name DESC;

CREATE OR REPLACE FUNCTION get_origin_v1(origin_name text) RETURNS SETOF origins
    LANGUAGE sql STABLE
    as $$
    SELECT * 
    FROM origins
    WHERE name = origin_name;
$$;

CREATE OR REPLACE VIEW origins_with_secret_key_full_name_v2 AS
  SELECT origins.id,
     origins.name,
     origins.owner_id,
     origin_secret_keys.full_name AS private_key_name,
     origins.default_package_visibility
    FROM (origins
      LEFT JOIN origin_secret_keys ON ((origins.id = origin_secret_keys.origin_id)))
   ORDER BY origins.id, origin_secret_keys.full_name DESC;


CREATE OR REPLACE FUNCTION insert_origin_v3(origin_name text, origin_owner_id bigint, origin_owner_name text, origin_default_package_visibility origin_package_visibility) RETURNS SETOF origins
    LANGUAGE plpgsql
    AS $$
  DECLARE
    inserted_origin origins;
  BEGIN
    INSERT INTO origins (name, owner_id, default_package_visibility)
      VALUES (origin_name, origin_owner_id, origin_default_package_visibility) RETURNING * into inserted_origin;
        PERFORM insert_origin_member_v1(inserted_origin.id, origin_name, origin_owner_id, origin_owner_name);
        PERFORM insert_origin_channel_v1(inserted_origin.id, origin_owner_id, 'unstable');
        PERFORM insert_origin_channel_v1(inserted_origin.id, origin_owner_id, 'stable');
        RETURN NEXT inserted_origin;
        RETURN;
      END
    $$;

CREATE OR REPLACE FUNCTION update_origin_v2(origin_name text, op_default_package_visibility origin_package_visibility) RETURNS void
    LANGUAGE sql
    AS $$
  UPDATE origins SET
    default_package_visibility = op_default_package_visibility,
    updated_at = now()
    WHERE name = origin_name;
$$;

CREATE OR REPLACE FUNCTION my_origins_with_stats_v2(om_account_id bigint) RETURNS TABLE(id bigint, name text, owner_id bigint, created_at timestamp with time zone, updated_at timestamp with time zone, default_package_visibility origin_package_visibility, package_count bigint)
    LANGUAGE sql STABLE
    AS $$
  SELECT o.*, count(op.id) as package_count
  FROM origins o
  INNER JOIN origin_members om ON o.id = om.origin_id
  LEFT OUTER JOIN origin_packages op on o.id = op.origin_id
  WHERE om.account_id = om_account_id
  GROUP BY o.id
  ORDER BY o.name;
$$;

