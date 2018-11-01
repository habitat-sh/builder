ALTER TABLE origin_packages ADD COLUMN ident_vector TSVECTOR;

UPDATE origin_packages SET ident_vector = to_tsvector(array_to_string(ident_array[1:2], ' '));

CREATE OR REPLACE FUNCTION update_origin_package_vector_index() RETURNS trigger AS $$
    DECLARE iws TEXT;
    BEGIN
        NEW.ident_vector := to_tsvector(array_to_string(NEW.ident_array[1:2], ' '));
        RETURN NEW;
    END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER origin_packages_vector BEFORE INSERT ON origin_packages FOR EACH ROW EXECUTE PROCEDURE update_origin_package_vector_index();
