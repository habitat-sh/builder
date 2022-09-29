DROP VIEW origin_packages_with_version_array;

ALTER TABLE origin_packages ADD COLUMN package_type text DEFAULT 'standard'::text NOT NULL;

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
