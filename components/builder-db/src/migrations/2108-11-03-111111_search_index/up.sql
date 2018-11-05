CREATE INDEX ident_index ON origin_packages USING gin(ident_vector);
