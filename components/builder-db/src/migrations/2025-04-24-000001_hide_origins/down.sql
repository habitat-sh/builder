ALTER TABLE origins DROP COLUMN hidden;
ALTER TABLE origin_packages DROP COLUMN hidden;
ALTER TABLE origin_package_settings DROP COLUMN hidden;

DROP VIEW origins_with_secret_key;
CREATE OR REPLACE VIEW origins_with_secret_key AS
  SELECT origins.name,
     origins.owner_id,
     origin_secret_keys.full_name AS private_key_name,
     origins.default_package_visibility,
     accounts.name AS owner_account
    FROM (origins
     LEFT JOIN origin_secret_keys ON ((origins.name = origin_secret_keys.origin))
     LEFT JOIN accounts ON ((origins.owner_id = accounts.id)))
   ORDER BY origins.name, origin_secret_keys.full_name DESC;

CREATE OR REPLACE VIEW origins_with_stats AS
    SELECT o.*, count(DISTINCT ops.name) as package_count
        FROM origins o
        LEFT OUTER JOIN origin_package_settings ops ON o.name = ops.origin
        GROUP BY o.name
        ORDER BY o.name;

DROP VIEW origin_packages_with_version_array;
CREATE OR REPLACE VIEW origin_packages_with_version_array AS
    SELECT
        id,
        owner_id,
        name,
        ident,
        ident_array,
        checksum,
        manifest,
        config,
        target,
        deps,
        tdeps,
        exposes,
        created_at,
        updated_at,
        visibility,
        origin,
        build_deps,
        build_tdeps,
        regexp_matches(ident_array[3], '([\d\.]*\d+)(.+)?') as version_array,
        package_type
FROM origin_packages;

DROP VIEW packages_with_channel_platform;
CREATE OR REPLACE VIEW packages_with_channel_platform AS
 SELECT op.id,
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
     JOIN origin_channel_packages ocp ON op.id = ocp.package_id
     JOIN origin_channels oc ON oc.id = ocp.channel_id
  WINDOW w AS (PARTITION BY op.origin, op.name, op.ident);

DROP VIEW origin_package_versions;
CREATE OR REPLACE VIEW origin_package_versions AS
    SELECT origin, name, visibility,
    ident_array[3] as version,
    COUNT(ident_array[4]) as release_count, 
    MAX(ident_array[4]) as latest,
    ARRAY_AGG(DISTINCT target) as platforms,
    regexp_matches(ident_array[3], '([\d\.]*\d+)(.+)?') as version_array
    FROM origin_packages
    GROUP BY ident_array[3], origin, name, visibility;
