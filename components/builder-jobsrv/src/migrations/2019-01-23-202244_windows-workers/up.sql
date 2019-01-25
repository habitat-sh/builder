ALTER TABLE busy_workers ADD COLUMN target Text DEFAULT 'x86_64-linux';
ALTER TABLE jobs ADD COLUMN target Text DEFAULT 'x86_64-linux';
ALTER TABLE jobs ALTER COLUMN job_state SET DEFAULT 'Pending';
ALTER TABLE groups ADD COLUMN target Text DEFAULT 'x86_64-linux';
ALTER TABLE group_projects ADD COLUMN target Text DEFAULT 'x86_64-linux';

CREATE OR REPLACE FUNCTION insert_group_v3(root_project text, project_names text[], project_idents text[], p_target text) RETURNS SETOF groups
    LANGUAGE sql
    AS $$
  WITH my_group AS (
          INSERT INTO groups (project_name, group_state, target)
          VALUES (root_project, 'Queued', p_target) RETURNING *
      ), my_project AS (
          INSERT INTO group_projects (owner_id, project_name, project_ident, project_state)
          SELECT g.id, project_info.name, project_info.ident, 'NotStarted'
          FROM my_group AS g, unnest(project_names, project_idents) AS project_info(name, ident)
      )
  SELECT * FROM my_group;
$$;

CREATE OR REPLACE FUNCTION insert_job_v3(p_owner_id bigint, p_project_id bigint, p_project_name text, p_project_owner_id bigint, p_project_plan_path text, p_vcs text, p_vcs_arguments text[], p_channel text, p_target text) RETURNS SETOF jobs
    LANGUAGE sql
    AS $$
      INSERT INTO jobs (owner_id, job_state, project_id, project_name, project_owner_id, project_plan_path, vcs, vcs_arguments, channel, target)
      VALUES (p_owner_id, 'Pending', p_project_id, p_project_name, p_project_owner_id, p_project_plan_path, p_vcs, p_vcs_arguments, p_channel, p_target)
      RETURNING *;
$$;

CREATE OR REPLACE FUNCTION next_pending_job_v2(p_worker text, p_target text) RETURNS SETOF jobs
    LANGUAGE plpgsql
    AS $$
DECLARE
    r jobs % rowtype;
BEGIN
    FOR r IN
        SELECT * FROM jobs
        WHERE job_state = 'Pending' AND target = p_target
        ORDER BY created_at ASC
        FOR UPDATE SKIP LOCKED
        LIMIT 1
    LOOP
        UPDATE jobs SET job_state='Dispatched', scheduler_sync=false, worker=p_worker, updated_at=now()
        WHERE id=r.id
        RETURNING * INTO r;
        RETURN NEXT r;
    END LOOP;
  RETURN;
END
$$;