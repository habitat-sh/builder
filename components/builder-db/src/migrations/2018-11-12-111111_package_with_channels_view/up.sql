-- Looks crazy to select all these, but we don't want to deal with the ts_vector
CREATE OR REPLACE VIEW packages_with_channel_platform AS
    SELECT 
        op.id,
        op.owner_id,
        op.name,
        op.ident,
        op.ident_array,
        op.checksum,
        op.manifest,
        op.config,
        op.target,
        op.deps,
        op.tdeps,
        op.exposes,
        op.visibility,
        op.created_at,
        op.updated_at,
        op.origin,
        array_agg(oc.name) OVER w AS channels,
        array_agg(op.target) OVER w AS platforms
    FROM origin_packages op
    INNER JOIN origin_channel_packages AS ocp ON op.id = ocp.package_id
    INNER JOIN origin_channels AS oc ON oc.id = ocp.channel_id
    WINDOW w AS (PARTITION BY op.ident);
