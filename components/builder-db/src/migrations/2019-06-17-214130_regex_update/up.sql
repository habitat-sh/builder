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
        regexp_matches(ident_array[3], '([\d\.]*\d+)(.+)?') as version_array
FROM origin_packages;