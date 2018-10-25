CREATE OR REPLACE FUNCTION get_origin_channel_package_latest_v8(op_origin text, op_channel text, op_ident_array text[], op_target text, op_visibilities origin_package_visibility[]) RETURNS SETOF origin_packages
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
      AND op.ident_array @> op_ident_array
      ORDER BY to_semver(ident_array[3]) desc, ident_array[4] desc
      LIMIT 1
$$;
