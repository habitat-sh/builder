CREATE OR REPLACE VIEW origins_with_secret_key AS
  SELECT origins.name,
     origins.owner_id,
     origin_secret_keys.full_name AS private_key_name,
     origins.default_package_visibility,
     accounts.name AS owner_account
    FROM (origins
     LEFT JOIN origin_secret_keys ON ((origins.name = origin_secret_keys.origin))
     LEFT JOIN accounts ON ((origins.owner_id = accounts.id)))
   ORDER BY origins.name, origin_secret_keys.full_name DESC;
