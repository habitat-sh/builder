ALTER TABLE origin_packages ALTER COLUMN build_deps SET DEFAULT '{}';
UPDATE origin_packages SET build_deps = DEFAULT where build_deps is NULL;
ALTER TABLE origin_packages ALTER COLUMN build_tdeps SET DEFAULT '{}';
UPDATE origin_packages SET build_tdeps = DEFAULT where build_tdeps is NULL;
