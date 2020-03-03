DROP VIEW origins_with_stats;

CREATE OR REPLACE VIEW origins_with_stats AS
    SELECT o.*, count(DISTINCT ops.name) as package_count
        FROM origins o
        LEFT OUTER JOIN origin_package_settings ops ON o.name = ops.origin
        GROUP BY o.name
        ORDER BY o.name;


