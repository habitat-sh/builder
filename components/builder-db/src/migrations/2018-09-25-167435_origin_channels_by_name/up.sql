CREATE OR REPLACE FUNCTION get_origin_channels_for_origin_v3(occ_origin_name text, occ_include_sandbox_channels boolean) RETURNS SETOF origin_channels
    LANGUAGE sql STABLE
    AS $$
    SELECT oc.*
    FROM origin_channels AS oc
    JOIN origins
    ON oc.origin_id = origins.id
    WHERE origins.name = occ_origin_name
    AND (occ_include_sandbox_channels = true OR (occ_include_sandbox_channels = false AND oc.name NOT LIKE 'bldr-%'))
    ORDER BY oc.name ASC;
$$;
