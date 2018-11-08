CREATE EXTENSION semver;

CREATE TYPE origin_package_visibility AS ENUM ('public', 'private', 'hidden');

ALTER TABLE origin_packages ALTER COLUMN visibility DROP DEFAULT;

ALTER TABLE origin_packages ALTER COLUMN visibility SET DATA TYPE origin_package_visibility
    USING visibility :: origin_package_visibility;

ALTER TABLE origin_packages ALTER COLUMN visibility SET DEFAULT 'public'::origin_package_visibility;

ALTER TABLE origin_packages ALTER COLUMN deps SET DATA TYPE text[]
    USING string_to_array(RTRIM(deps, ':'), ':') :: text[];

ALTER TABLE origin_packages ALTER COLUMN tdeps SET DATA TYPE text[]
    USING string_to_array(RTRIM(tdeps, ':'), ':') :: text[];

ALTER TABLE origin_packages ALTER COLUMN exposes SET DATA TYPE integer[]
    USING string_to_array(RTRIM(exposes, ':'), ':') :: integer[];

CREATE OR REPLACE FUNCTION get_origin_package_v5(op_ident text, op_visibilities origin_package_visibility[]) RETURNS SETOF origin_packages
    LANGUAGE sql STABLE
    AS $$
    SELECT *
    FROM origin_packages
    WHERE ident = op_ident
    AND visibility = ANY(op_visibilities);
$$;
