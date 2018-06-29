ALTER TABLE IF EXISTS origin_projects ADD COLUMN IF NOT EXISTS auto_build bool NOT NULL DEFAULT true;

CREATE OR REPLACE FUNCTION update_origin_project_v4 (
  project_id bigint,
  project_origin_id bigint,
  project_package_name text,
  project_plan_path text,
  project_vcs_type text,
  project_vcs_data text,
  project_owner_id bigint,
  project_vcs_installation_id bigint,
  project_visibility text,
  project_auto_build bool
) RETURNS void AS $$
    BEGIN
      UPDATE origin_projects SET
          package_name = project_package_name,
          name = (SELECT name FROM origins WHERE id = project_origin_id) || '/' || project_package_name,
          plan_path = project_plan_path,
          vcs_type = project_vcs_type,
          vcs_data = project_vcs_data,
          owner_id = project_owner_id,
          updated_at = now(),
          vcs_installation_id = project_vcs_installation_id,
          visibility = project_visibility,
          auto_build = project_auto_build
          WHERE id = project_id;
    END
$$ LANGUAGE plpgsql VOLATILE;

CREATE OR REPLACE FUNCTION insert_origin_project_v5 (
  project_origin_name text,
  project_package_name text,
  project_plan_path text,
  project_vcs_type text,
  project_vcs_data text,
  project_owner_id bigint,
  project_vcs_installation_id bigint,
  project_visibility text,
  project_auto_build bool
) RETURNS SETOF origin_projects AS $$
    BEGIN
      RETURN QUERY INSERT INTO origin_projects (origin_id,
                                  origin_name,
                                  package_name,
                                  name,
                                  plan_path,
                                  owner_id,
                                  vcs_type,
                                  vcs_data,
                                  vcs_installation_id,
                                  visibility,
                                  auto_build)
            VALUES (
                (SELECT id FROM origins where name = project_origin_name),
                project_origin_name,
                project_package_name,
                project_origin_name || '/' || project_package_name,
                project_plan_path,
                project_owner_id,
                project_vcs_type,
                project_vcs_data,
                project_vcs_installation_id,
                project_visibility,
                project_auto_build)
            RETURNING *;
        RETURN;
    END
$$ LANGUAGE plpgsql VOLATILE;
