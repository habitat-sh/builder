CREATE OR REPLACE VIEW origins_with_stats AS
    SELECT o.*, count(DISTINCT op.name) as package_count
        FROM origins o
        LEFT OUTER JOIN origin_packages op ON o.name = op.origin
        group by o.name
        ORDER BY o.name DESC;
