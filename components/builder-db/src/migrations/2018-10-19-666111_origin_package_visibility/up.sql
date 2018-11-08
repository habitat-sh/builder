DROP VIEW IF EXISTS origins_with_private_encryption_key_full_name_v1;
DROP VIEW IF EXISTS origins_with_secret_key_full_name_v2;

ALTER TABLE origins ALTER COLUMN default_package_visibility DROP DEFAULT;
ALTER TABLE origins ALTER COLUMN default_package_visibility SET DATA TYPE origin_package_visibility
    USING default_package_visibility :: origin_package_visibility;
ALTER TABLE origins ALTER COLUMN default_package_visibility SET DEFAULT 'public'::origin_package_visibility;
