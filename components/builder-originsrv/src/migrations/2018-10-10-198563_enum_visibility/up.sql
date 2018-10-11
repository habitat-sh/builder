CREATE EXTENSION semver;

CREATE TYPE origin_package_visibility AS ENUM ('public', 'private', 'hidden');

ALTER TABLE origin_packages ALTER COLUMN visibility DROP DEFAULT;

ALTER TABLE origin_packages ALTER COLUMN visibility SET DATA TYPE origin_package_visibility
    USING visibility :: origin_package_visibility;

ALTER TABLE origin_packages ALTER COLUMN visibility SET DEFAULT 'public'::origin_package_visibility;

ALTER TABLE origin_packages ALTER COLUMN deps SET DATA TYPE text[]
    USING string_to_array(deps, ':') :: text[];

ALTER TABLE origin_packages ALTER COLUMN tdeps SET DATA TYPE text[]
    USING string_to_array(tdeps, ':') :: text[];

ALTER TABLE origin_packages ALTER COLUMN exposes SET DATA TYPE smallint[]
    USING string_to_array(exposes, ':') :: smallint[];

CREATE OR REPLACE FUNCTION get_origin_channel_package_latest_v6(op_origin text, op_channel text, op_ident text, op_target text, op_visibilities origin_package_visibility[]) RETURNS SETOF origin_packages
    LANGUAGE sql STABLE
    AS $$
      SELECT op.*
      FROM origin_packages op
      INNER JOIN origin_channel_packages ocp on ocp.package_id = op.id
      INNER JOIN origin_channels oc on ocp.channel_id = oc.id
      INNER JOIN origins o on oc.origin_id = o.id
      WHERE o.name = op_origin
      AND oc.name = op_channel
      AND op.target = op_target
      AND op.visibility = ANY(op_visibilities)
      AND op.ident LIKE (op_ident  || '%');
$$;

CREATE OR REPLACE FUNCTION get_origin_channel_package_v5(op_origin text, op_channel text, op_ident text, op_visibilities origin_package_visibility[]) RETURNS SETOF origin_packages
    LANGUAGE sql STABLE
    AS $$
      SELECT op.*
      FROM origin_packages op
      INNER JOIN origin_channel_packages ocp on ocp.package_id = op.id
      INNER JOIN origin_channels oc on ocp.channel_id = oc.id
      INNER JOIN origins o on oc.origin_id = o.id
      WHERE op.ident = op_ident
      AND o.name = op_origin
      AND oc.name = op_channel
      AND op.visibility = ANY(op_visibilities);
$$;

CREATE OR REPLACE FUNCTION get_origin_channel_packages_for_channel_v4(op_origin text, op_channel text, op_ident text, op_visibilities origin_package_visibility[], op_limit bigint, op_offset bigint) RETURNS TABLE(total_count bigint, ident text)
    LANGUAGE sql STABLE
    AS $$
      SELECT COUNT(*) OVER () AS total_count, op.ident
      FROM origin_packages op
      INNER JOIN origin_channel_packages ocp on ocp.package_id = op.id
      INNER JOIN origin_channels oc on ocp.channel_id = oc.id
      INNER JOIN origins o on oc.origin_id = o.id
      WHERE o.name = op_origin
      AND oc.name = op_channel
      AND op.visibility = ANY(op_visibilities)
      AND op.ident LIKE (op_ident  || '%')
      ORDER BY ident ASC
      LIMIT op_limit OFFSET op_offset;
$$;

CREATE OR REPLACE FUNCTION get_origin_package_channels_for_package_v5(op_ident text, op_visibilities origin_package_visibility[]) RETURNS SETOF origin_channels
    LANGUAGE sql STABLE
    AS $$
        SELECT oc.*
          FROM origin_channels oc INNER JOIN origin_channel_packages ocp ON oc.id = ocp.channel_id
          INNER JOIN origin_packages op ON op.id = ocp.package_id
          WHERE op.ident = op_ident
          AND op.visibility = ANY(op_visibilities)
          ORDER BY oc.name;
$$;

CREATE OR REPLACE FUNCTION get_origin_package_latest_v6(op_ident text, op_target text, op_visibilities origin_package_visibility[]) RETURNS SETOF origin_packages
    LANGUAGE sql STABLE
    AS $$
      SELECT *
      FROM origin_packages
      WHERE ident LIKE (op_ident  || '%')
      AND target = op_target
      AND visibility = ANY(op_visibilities)
      ORDER BY to_semver(ident_array[3]) desc, ident_array[4] desc
      LIMIT 1;
$$;

CREATE OR REPLACE FUNCTION get_origin_package_platforms_for_package_v5(op_ident text, op_visibilities origin_package_visibility[]) RETURNS TABLE(target text)
    LANGUAGE sql STABLE
    AS $$
  SELECT DISTINCT target
  FROM origin_packages
  WHERE ident LIKE (op_ident || '%')
  AND visibility = ANY(op_visibilities)
$$;

CREATE OR REPLACE FUNCTION get_origin_package_v5(op_ident text, op_visibilities origin_package_visibility[]) RETURNS SETOF origin_packages
    LANGUAGE sql STABLE
    AS $$
    SELECT *
    FROM origin_packages
    WHERE ident = op_ident
    AND visibility = ANY(op_visibilities);
$$;

CREATE OR REPLACE FUNCTION get_origin_package_versions_for_origin_v8(op_origin text, op_pkg text, op_visibilities origin_package_visibility[]) RETURNS TABLE(version text, release_count bigint, latest text, platforms text)
    LANGUAGE sql STABLE
    AS $$
  WITH packages AS (
    SELECT *
    FROM origin_packages op INNER JOIN origins o ON o.id = op.origin_id
    WHERE o.name = op_origin
    AND op.name = op_pkg
    AND op.visibility = ANY(op_visibilities)
  ), idents AS (
    SELECT regexp_split_to_array(ident, '/') as parts, target
    FROM packages
  )
  SELECT i.parts[3] AS version,
  COUNT(i.parts[4]) AS release_count,
  MAX(i.parts[4]) as latest,
  ARRAY_TO_STRING(ARRAY_AGG(DISTINCT i.target), ',')
  FROM idents i
  GROUP BY version
  ORDER BY version DESC
$$;

CREATE OR REPLACE FUNCTION get_origin_packages_for_origin_distinct_v5(op_ident text, op_limit bigint, op_offset bigint, op_visibilities origin_package_visibility[]) RETURNS TABLE(total_count bigint, ident text)
    LANGUAGE sql STABLE
    AS $$
    SELECT COUNT(p.partial_ident[1] || '/' || p.partial_ident[2]) OVER () AS total_count, p.partial_ident[1] || '/' || p.partial_ident[2] AS ident
    FROM (SELECT regexp_split_to_array(op.ident, '/') as partial_ident
          FROM origin_packages op
          WHERE op.ident LIKE ('%' || op_ident || '%')
          AND op.visibility = ANY(op_visibilities)
          ) AS p
    GROUP BY (p.partial_ident[1] || '/' || p.partial_ident[2])
    LIMIT op_limit
    OFFSET op_offset;
$$;

CREATE OR REPLACE FUNCTION get_origin_packages_for_origin_v6(op_ident text, op_limit bigint, op_offset bigint, op_visibilities origin_package_visibility[]) RETURNS TABLE(total_count bigint, ident text)
    LANGUAGE sql STABLE
    AS $$
        SELECT COUNT(*) OVER () AS total_count, op.ident
        FROM origin_packages op
        WHERE op.ident LIKE (op_ident  || '%')
        AND op.visibility = ANY(op_visibilities)
        ORDER BY op.ident DESC
        LIMIT op_limit
        OFFSET op_offset;
$$;

CREATE OR REPLACE FUNCTION get_origin_packages_unique_for_origin_v5(op_origin text, op_limit bigint, op_offset bigint, op_visibilities origin_package_visibility[]) RETURNS TABLE(total_count bigint, name text)
    LANGUAGE sql STABLE
    AS $$
        SELECT COUNT(*) OVER () AS total_count, op.name
        FROM origins o INNER JOIN origin_packages op ON o.id = op.origin_id
        WHERE o.name = op_origin
        AND op.visibility = ANY(op_visibilities)
        GROUP BY op.name
        ORDER BY op.name ASC
        LIMIT op_limit
        OFFSET op_offset;
$$;

CREATE OR REPLACE FUNCTION insert_origin_package_v5 (
  op_origin_id bigint,
  op_owner_id bigint,
  op_name text,
  op_ident text,
  op_checksum text,
  op_manifest text,
  op_config text,
  op_target text,
  op_deps text[],
  op_tdeps text[],
  op_exposes smallint[],
  op_visibility origin_package_visibility
) RETURNS SETOF origin_packages AS $$
    DECLARE
      inserted_package origin_packages;
      channel_id bigint;
    BEGIN
        INSERT INTO origin_packages (origin_id, owner_id, name, ident, ident_array, checksum, manifest, config, target, deps, tdeps, exposes, visibility)
              VALUES (op_origin_id, op_owner_id, op_name, op_ident, regexp_split_to_array(op_ident, '/'), op_checksum, op_manifest, op_config, op_target, op_deps, op_tdeps, op_exposes, op_visibility)
              ON CONFLICT ON CONSTRAINT origin_packages_ident_key DO
                UPDATE set checksum=op_checksum
              RETURNING * into inserted_package;

        SELECT id FROM origin_channels WHERE origin_id = op_origin_id AND name = 'unstable' INTO channel_id;
        PERFORM promote_origin_package_v1(channel_id, inserted_package.id);

        RETURN NEXT inserted_package;
        RETURN;
    END
$$ LANGUAGE plpgsql VOLATILE;


CREATE OR REPLACE FUNCTION add_audit_package_entry_v3(p_origin text, p_package text, p_channel text, p_operation origin_package_operation, p_trigger package_channel_trigger, p_requester_id bigint, p_requester_name text) RETURNS SETOF audit_package
    LANGUAGE sql
    AS $$
INSERT INTO audit_package (origin_id, package_id, channel_id, operation, trigger, requester_id, requester_name)
VALUES (
    (SELECT id FROM origins where name = p_origin),
    (SELECT id FROM get_origin_package_v5(p_package, '{public,private,hidden}')),
    (SELECT id FROM get_origin_channel_v1(p_origin, p_channel)),
    p_operation, p_trigger, p_requester_id, p_requester_name)
RETURNING *;
$$;

CREATE OR REPLACE FUNCTION promote_origin_package_v3(in_origin text, in_ident text, to_channel text) RETURNS void
    LANGUAGE sql
    AS $$
    INSERT INTO origin_channel_packages (channel_id, package_id)
    VALUES (
        (SELECT id from get_origin_channel_v1(in_origin, to_channel)),
        (SELECT id from get_origin_package_v5(in_ident, '{public,private,hidden}'))
    );
$$;
CREATE OR REPLACE FUNCTION demote_origin_package_v3(in_origin text, in_ident text, out_channel text) RETURNS void
    LANGUAGE sql
    AS $$
      DELETE FROM origin_channel_packages
      WHERE channel_id=(SELECT id from get_origin_channel_v1(in_origin, out_channel))
      AND package_id=(SELECT id from get_origin_package_v5(in_ident, '{public,private,hidden}'));
$$;

CREATE OR REPLACE FUNCTION update_package_visibility_in_bulk_v2(op_visibility origin_package_visibility, op_ids bigint[]) RETURNS void
    LANGUAGE sql
    AS $$
    UPDATE origin_packages
    SET visibility = op_visibility
    WHERE id IN (SELECT(unnest(op_ids)));
$$;

CREATE OR REPLACE FUNCTION update_origin_package_v2(op_id bigint, op_owner_id bigint, op_name text, op_ident text, op_checksum text, op_manifest text, op_config text, op_target text, op_deps text[], op_tdeps text[], op_exposes smallint[], op_visibility origin_package_visibility) RETURNS void
    LANGUAGE sql
    AS $$
  UPDATE origin_packages SET
    owner_id = op_owner_id,
    name = op_name,
    ident = op_ident,
    checksum = op_checksum,
    manifest = op_manifest,
    config = op_config,
    target = op_target,
    deps = op_deps,
    tdeps = op_tdeps,
    exposes = op_exposes,
    visibility = op_visibility,
    scheduler_sync = false,
    updated_at = now()
    WHERE id = op_id;
$$;
