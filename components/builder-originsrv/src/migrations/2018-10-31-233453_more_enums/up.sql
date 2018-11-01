ALTER TABLE audit_package_group ALTER COLUMN operation SET DATA TYPE origin_package_operation
    USING CASE operation
        WHEN 0 THEN 'promote'
        WHEN 1 THEN 'demote'
        ELSE NULL
    END :: origin_package_operation;

ALTER TABLE audit_package_group ALTER COLUMN trigger SET DATA TYPE package_channel_trigger
    USING CASE trigger
        WHEN 0 THEN 'unknown'
        WHEN 1 THEN 'builder_ui'
        WHEN 2 THEN 'hab_client'
        ELSE NULL
    END :: package_channel_trigger;

ALTER TABLE origin_projects ALTER COLUMN visibility DROP DEFAULT;

ALTER TABLE origin_projects ALTER COLUMN visibility SET DATA TYPE origin_package_visibility
    USING visibility :: origin_package_visibility;

ALTER TABLE origin_projects ALTER COLUMN visibility SET DEFAULT 'public'::origin_package_visibility;

CREATE OR REPLACE FUNCTION add_audit_package_group_entry_v2(p_origin text, p_channel text, p_package_ids bigint[], p_operation origin_package_operation, p_trigger package_channel_trigger, p_requester_id bigint, p_requester_name text, p_group_id bigint) RETURNS SETOF audit_package_group
    LANGUAGE sql
    AS $$
INSERT INTO audit_package_group (origin_id, channel_id, package_ids, operation, trigger, requester_id, requester_name, group_id)
VALUES (
    (SELECT id FROM origins where name = p_origin),
    (SELECT id FROM get_origin_channel_v1(p_origin, p_channel)),
    p_package_ids, p_operation, p_trigger, p_requester_id, p_requester_name, p_group_id)
RETURNING *;
$$;

CREATE OR REPLACE FUNCTION insert_origin_project_v6(project_origin_name text, project_package_name text, project_plan_path text, project_vcs_type text, project_vcs_data text, project_owner_id bigint, project_vcs_installation_id bigint, project_visibility origin_package_visibility, project_auto_build boolean) RETURNS SETOF origin_projects
    LANGUAGE plpgsql
    AS $$
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
$$;

CREATE OR REPLACE FUNCTION update_origin_project_v5(project_id bigint, project_origin_id bigint, project_package_name text, project_plan_path text, project_vcs_type text, project_vcs_data text, project_owner_id bigint, project_vcs_installation_id bigint, project_visibility origin_package_visibility, project_auto_build boolean) RETURNS void
    LANGUAGE plpgsql
    AS $$
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
$$;