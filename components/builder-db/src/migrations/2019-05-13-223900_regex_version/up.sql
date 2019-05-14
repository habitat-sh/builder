CREATE OR REPLACE VIEW origin_package_versions AS
    SELECT origin, name, visibility,
    ident_array[3] as version,
    COUNT(ident_array[4]) as release_count, 
    MAX(ident_array[4]) as latest,
    ARRAY_AGG(DISTINCT target) as platforms,
    regexp_matches(ident_array[3], '([\d\.]+)(.+)?') as version_array
    FROM origin_packages
    GROUP BY ident_array[3], origin, name, visibility;

CREATE OR REPLACE VIEW origin_packages_with_version_array AS
    SELECT
        id,
        ident_array,
        regexp_matches(ident_array[3], '([\d\.]+)(.+)?') as version_array
    FROM origin_packages;

DROP EXTENSION semver;