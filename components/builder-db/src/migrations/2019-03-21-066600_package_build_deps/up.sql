ALTER TABLE origin_packages ADD COLUMN build_deps text[];
ALTER TABLE origin_packages ADD COLUMN build_tdeps text[];
DROP VIEW packages_with_channel_platform;
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
        op.build_deps,
        op.build_tdeps,
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



