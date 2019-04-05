ALTER TABLE origin_packages DROP CONSTRAINT origin_packages_ident_key;
ALTER TABLE origin_packages ADD CONSTRAINT origin_packages_ident_target_key UNIQUE(ident, target);