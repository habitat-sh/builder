ALTER TABLE origin_channels ADD COLUMN origin text REFERENCES origins(name);
UPDATE origin_channels set origin = origins.name FROM origins where origin_channels.origin_id = origins.id;
ALTER TABLE origin_channels DROP COLUMN origin_id;
ALTER TABLE origin_channels ADD UNIQUE(origin, name);

ALTER TABLE origin_secrets ADD COLUMN origin text REFERENCES origins(name);
UPDATE origin_secrets set origin = origins.name FROM origins where origin_secrets.origin_id = origins.id;
ALTER TABLE origin_secrets DROP COLUMN origin_id;

ALTER TABLE origin_secret_keys ADD COLUMN origin text REFERENCES origins(name);
UPDATE origin_secret_keys set origin = origins.name FROM origins where origin_secret_keys.origin_id = origins.id;
ALTER TABLE origin_secret_keys DROP COLUMN origin_id CASCADE; -- VIEW CASCADE HERE origins_with_private_encryption_key_full_name_v1

ALTER TABLE origin_public_keys ADD COLUMN origin text REFERENCES origins(name);
UPDATE origin_public_keys set origin = origins.name FROM origins where origin_public_keys.origin_id = origins.id;
ALTER TABLE origin_public_keys DROP COLUMN origin_id;

ALTER TABLE origin_public_encryption_keys ADD COLUMN origin text REFERENCES origins(name);
UPDATE origin_public_encryption_keys set origin = origins.name FROM origins where origin_public_encryption_keys.origin_id = origins.id;
ALTER TABLE origin_public_encryption_keys DROP COLUMN origin_id;

ALTER TABLE origin_private_encryption_keys ADD COLUMN origin text REFERENCES origins(name);
UPDATE origin_private_encryption_keys set origin = origins.name FROM origins where origin_private_encryption_keys.origin_id = origins.id;
ALTER TABLE origin_private_encryption_keys DROP COLUMN origin_id CASCADE; -- VIEW CASCADE HERE origins_with_secret_key_full_name_v2

ALTER TABLE origin_packages ADD COLUMN origin text REFERENCES origins(name);
UPDATE origin_packages set origin = origins.name FROM origins where origin_packages.origin_id = origins.id;
ALTER TABLE origin_packages DROP COLUMN origin_id;

ALTER TABLE audit_package_group ADD COLUMN origin text REFERENCES origins(name);
UPDATE audit_package_group set origin = origins.name FROM origins where audit_package_group.origin_id = origins.id;
ALTER TABLE audit_package_group DROP COLUMN origin_id;

ALTER TABLE audit_package_group ADD COLUMN channel text;
UPDATE audit_package_group set channel = origin_channels.name FROM origin_channels where audit_package_group.channel_id = origin_channels.id;
ALTER TABLE audit_package_group DROP COLUMN channel_id;

ALTER TABLE audit_package ADD COLUMN origin text REFERENCES origins(name);
UPDATE audit_package set origin = origins.name FROM origins where audit_package.origin_id = origins.id;
ALTER TABLE audit_package DROP COLUMN origin_id;

ALTER TABLE audit_package ADD COLUMN channel text;
UPDATE audit_package set channel = origin_channels.name FROM origin_channels where audit_package.channel_id = origin_channels.id;
ALTER TABLE audit_package DROP COLUMN channel_id;

ALTER TABLE audit_package ADD COLUMN package_ident text;
UPDATE audit_package set package_ident = origin_packages.ident FROM origin_packages where audit_package.package_id = origin_packages.id;
ALTER TABLE audit_package DROP COLUMN package_id;

ALTER TABLE origin_invitations RENAME COLUMN origin_name TO origin; 
ALTER TABLE origin_invitations ADD CONSTRAINT origin_invitations_origin_fkey FOREIGN KEY (origin) REFERENCES origins(name);
ALTER TABLE origin_invitations DROP COLUMN origin_id;

ALTER TABLE origin_projects RENAME COLUMN origin_name TO origin; 
ALTER TABLE origin_projects ADD CONSTRAINT origin_projects_origin_fkey FOREIGN KEY (origin) REFERENCES origins(name);
ALTER TABLE origin_projects DROP COLUMN origin_id;
ALTER TABLE origin_projects DROP COLUMN vcs_auth_token;
ALTER TABLE origin_projects DROP COLUMN vcs_username;

ALTER TABLE origin_members RENAME COLUMN origin_name TO origin; 
ALTER TABLE origin_members ADD CONSTRAINT origin_members_origin_fkey FOREIGN KEY (origin) REFERENCES origins(name);
ALTER TABLE origin_members DROP COLUMN origin_id;
ALTER TABLE origin_members DROP COLUMN account_name;
-- TODO - Add primary key back for (origin_name, account_id) ?

ALTER TABLE origins DROP COLUMN id;
ALTER TABLE origins ADD PRIMARY KEY (name);

CREATE OR REPLACE VIEW origins_with_secret_key AS
  SELECT origins.name,
     origins.owner_id,
     origin_secret_keys.full_name AS private_key_name,
     origins.default_package_visibility
    FROM (origins
      LEFT JOIN origin_secret_keys ON ((origins.name = origin_secret_keys.origin)))
   ORDER BY origins.name, origin_secret_keys.full_name DESC;

CREATE OR REPLACE VIEW origins_with_stats AS
    SELECT o.*, count(*) as package_count
        FROM origins o
        LEFT OUTER JOIN origin_packages op ON o.name = op.origin
        group by o.name, op.name
        ORDER BY o.name DESC;

CREATE OR REPLACE VIEW origin_package_versions AS
    SELECT origin, name, visibility, ident_array[3] as version, count(ident_array[4]) as release_count, 
    MAX(ident_array[4]) as latest, ARRAY_AGG(DISTINCT target) as platforms
    FROM origin_packages
    GROUP BY ident_array[3], origin, name, visibility;
