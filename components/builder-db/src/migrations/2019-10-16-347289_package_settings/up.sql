START TRANSACTION;
  ALTER TABLE origin_projects ADD COLUMN IF NOT EXISTS target text;
  ALTER TABLE origin_projects DROP CONSTRAINT "origin_projects_origin_name_package_name_name_key";
  ALTER TABLE origin_projects ADD CONSTRAINT "origin_projects_origin_name_package_name_name_target_key" 
    UNIQUE (origin,package_name,name,target);
  ALTER TABLE origin_projects DROP COLUMN IF EXISTS visibility;
COMMIT;

CREATE SEQUENCE IF NOT EXISTS origin_package_settings_id_seq;
CREATE TABLE IF NOT EXISTS origin_package_settings (
    id bigint DEFAULT next_id_v1('origin_package_settings_id_seq') PRIMARY KEY NOT NULL,
    origin text,
    package_name text,
    visibility origin_package_visibility,
    owner_id bigint,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    UNIQUE (origin, package_name)
);

START TRANSACTION; 
  INSERT into origin_package_settings(origin, package_name, visibility, owner_id)
    SELECT DISTINCT ON (origin, name) origin, name, visibility, owner_id 
    FROM origin_packages;

  UPDATE origin_projects SET target=existing.target
    FROM ( 
      SELECT DISTINCT ON (origin,name) origin, name, target 
      FROM origin_packages) as existing
    WHERE origin_projects.origin = existing.origin
    AND origin_projects.package_name = existing.name;

COMMIT;
