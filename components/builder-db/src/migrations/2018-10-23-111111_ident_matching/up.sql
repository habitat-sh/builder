CREATE OR REPLACE FUNCTION get_origin_channel_package_latest_v7(op_origin text, op_channel text, op_ident_array text[], op_target text, op_visibilities origin_package_visibility[]) RETURNS SETOF origin_packages
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
      AND op.ident_array @> op_ident_array;
$$;

CREATE OR REPLACE FUNCTION get_origin_channel_packages_for_channel_v5(op_origin text, op_channel text, op_ident_array text[], op_visibilities origin_package_visibility[], op_limit bigint, op_offset bigint) RETURNS TABLE(total_count bigint, ident text)
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
      AND op.ident_array @> op_ident_array
      ORDER BY ident ASC
      LIMIT op_limit OFFSET op_offset;
$$;

CREATE OR REPLACE FUNCTION get_origin_package_latest_v7(op_ident_array text[], op_target text, op_visibilities origin_package_visibility[]) RETURNS SETOF origin_packages
    LANGUAGE sql STABLE
    AS $$
      SELECT *
      FROM origin_packages
      WHERE ident_array @> op_ident_array
      AND target = op_target
      AND visibility = ANY(op_visibilities)
      ORDER BY to_semver(ident_array[3]) desc, ident_array[4] desc
      LIMIT 1;
$$;

CREATE OR REPLACE FUNCTION get_origin_package_platforms_for_package_v6(op_ident_array text[], op_visibilities origin_package_visibility[]) RETURNS TABLE(target text)
    LANGUAGE sql STABLE
    AS $$
  SELECT DISTINCT target
  FROM origin_packages
  WHERE ident_array @> op_ident_array
  AND visibility = ANY(op_visibilities)
$$;

CREATE OR REPLACE FUNCTION get_origin_packages_for_origin_distinct_v6(op_ident_array text[], op_limit bigint, op_offset bigint, op_visibilities origin_package_visibility[]) RETURNS TABLE(total_count bigint, ident text)
    LANGUAGE sql STABLE
    AS $$
    SELECT COUNT(p.partial_ident[1] || '/' || p.partial_ident[2]) OVER () AS total_count, p.partial_ident[1] || '/' || p.partial_ident[2] AS ident
    FROM (SELECT regexp_split_to_array(op.ident, '/') as partial_ident
          FROM origin_packages op
          WHERE ident_array @> op_ident_array
          AND op.visibility = ANY(op_visibilities)
          ) AS p
    GROUP BY (p.partial_ident[1] || '/' || p.partial_ident[2])
    LIMIT op_limit
    OFFSET op_offset;
$$;

CREATE OR REPLACE FUNCTION get_origin_packages_for_origin_v7(op_ident_array text[], op_limit bigint, op_offset bigint, op_visibilities origin_package_visibility[]) RETURNS TABLE(total_count bigint, ident text)
    LANGUAGE sql STABLE
    AS $$
        SELECT COUNT(*) OVER () AS total_count, op.ident
        FROM origin_packages op
        WHERE op.ident_array @> op_ident_array
        AND op.visibility = ANY(op_visibilities)
        ORDER BY op.ident DESC
        LIMIT op_limit
        OFFSET op_offset;
$$;
