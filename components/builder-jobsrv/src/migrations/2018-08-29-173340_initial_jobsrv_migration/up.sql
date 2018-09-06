CREATE EXTENSION IF NOT EXISTS plpgsql WITH SCHEMA pg_catalog;
CREATE SEQUENCE IF NOT EXISTS groups_id_seq;
CREATE SEQUENCE IF NOT EXISTS job_id_seq;
CREATE SEQUENCE IF NOT EXISTS group_projects_id_seq;

CREATE TABLE IF NOT EXISTS audit_jobs (
    group_id bigint,
    operation smallint,
    trigger smallint,
    requester_id bigint,
    requester_name text,
    created_at timestamp with time zone DEFAULT now()
);

CREATE TABLE IF NOT EXISTS groups (
    id bigint DEFAULT next_id_v1('groups_id_seq') PRIMARY KEY NOT NULL,
    group_state text,
    project_name text,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now()
);

CREATE TABLE IF NOT EXISTS group_projects (
    id bigint NOT NULL DEFAULT nextval('group_projects_id_seq') PRIMARY KEY,
    owner_id bigint,
    project_name text,
    project_ident text,
    project_state text,
    job_id bigint DEFAULT 0,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now()
);

CREATE TABLE IF NOT EXISTS jobs (
    id bigint DEFAULT next_id_v1('job_id_seq') PRIMARY KEY NOT NULL,
    owner_id bigint,
    job_state text,
    project_id bigint,
    project_name text,
    project_owner_id bigint,
    project_plan_path text,
    vcs text,
    vcs_arguments text[],
    net_error_code integer,
    net_error_msg text,
    scheduler_sync boolean DEFAULT false,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    build_started_at timestamp with time zone,
    build_finished_at timestamp with time zone,
    package_ident text,
    archived boolean DEFAULT false NOT NULL,
    channel text,
    sync_count integer DEFAULT 0,
    worker text
);

CREATE TABLE IF NOT EXISTS busy_workers (
    ident text,
    job_id bigint,
    quarantined boolean,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    UNIQUE (ident, job_id)
);

CREATE OR REPLACE FUNCTION abort_group_v1(in_gid bigint) RETURNS void
    LANGUAGE sql
    AS $$
  UPDATE group_projects SET project_state='Failure'
    WHERE owner_id = in_gid
    AND (project_state = 'InProgress' OR project_state = 'NotStarted');
  UPDATE groups SET group_state='Complete' where id = in_gid;
$$;

CREATE OR REPLACE FUNCTION add_audit_jobs_entry_v1(p_group_id bigint, p_operation smallint, p_trigger smallint, p_requester_id bigint, p_requester_name text) RETURNS SETOF audit_jobs
    LANGUAGE sql
    AS $$
      INSERT INTO audit_jobs (group_id, operation, trigger, requester_id, requester_name)
      VALUES (p_group_id, p_operation, p_trigger, p_requester_id, p_requester_name)
      RETURNING *;
$$;

CREATE OR REPLACE FUNCTION cancel_group_v1(in_gid bigint) RETURNS void
    LANGUAGE sql
    AS $$
  UPDATE group_projects SET project_state='Canceled'
    WHERE owner_id = in_gid
    AND (project_state = 'NotStarted');
  UPDATE groups SET group_state='Canceled' where id = in_gid;
$$;

CREATE OR REPLACE FUNCTION check_active_group_v1(pname text) RETURNS SETOF groups
    LANGUAGE sql
    AS $$
  SELECT * FROM groups
  WHERE project_name = pname
  AND group_state IN ('Pending', 'Dispatching')
$$;

CREATE OR REPLACE FUNCTION count_group_projects_v2(origin text) RETURNS bigint
    LANGUAGE plpgsql STABLE
    AS $$
  BEGIN
    RETURN COUNT(*) FROM group_projects WHERE project_ident LIKE (origin || '/%');
  END
$$;

-- TED these two functions can be removed when the stats endpoint is gone
CREATE OR REPLACE FUNCTION count_origin_packages_v1(origin text) RETURNS bigint
    LANGUAGE plpgsql STABLE
    AS $$
BEGIN
  RETURN COUNT(*) FROM origin_packages WHERE ident_array[1]=origin;
END
$$;

CREATE OR REPLACE FUNCTION count_unique_origin_packages_v1(op_origin text) RETURNS bigint
    LANGUAGE sql STABLE
    AS $$
  SELECT COUNT(DISTINCT ident_array[2]) AS total
  FROM origin_packages
  WHERE ident_array[1] = op_origin
$$;

CREATE OR REPLACE FUNCTION delete_busy_worker_v1(in_ident text, in_job_id bigint) RETURNS void
    LANGUAGE sql
    AS $$
  DELETE FROM busy_workers
  WHERE ident = in_ident AND job_id = in_job_id
$$;

CREATE OR REPLACE FUNCTION find_group_project_v1(gid bigint, name text) RETURNS SETOF group_projects
    LANGUAGE plpgsql STABLE
    AS $$
BEGIN
  RETURN QUERY SELECT * FROM group_projects WHERE owner_id = gid AND project_name = name;
  RETURN;
END
$$;

CREATE OR REPLACE FUNCTION get_busy_workers_v1() RETURNS SETOF busy_workers
    LANGUAGE sql STABLE
    AS $$
  SELECT * FROM busy_workers
$$;

CREATE OR REPLACE FUNCTION get_cancel_pending_jobs_v1() RETURNS SETOF jobs
    LANGUAGE sql
    AS $$
  SELECT *
  FROM jobs
  WHERE job_state = 'CancelPending'
$$;

CREATE OR REPLACE FUNCTION get_dispatched_jobs_v1() RETURNS SETOF jobs
    LANGUAGE sql STABLE
    AS $$
  SELECT *
  FROM jobs
  WHERE job_state = 'Dispatched'
$$;

CREATE OR REPLACE FUNCTION get_group_projects_for_group_v1(gid bigint) RETURNS SETOF group_projects
    LANGUAGE plpgsql STABLE
    AS $$
  BEGIN
    RETURN QUERY SELECT * FROM group_projects WHERE owner_id = gid;
    RETURN;
  END
$$;

CREATE OR REPLACE FUNCTION get_group_v1(gid bigint) RETURNS SETOF groups
    LANGUAGE plpgsql STABLE
    AS $$
BEGIN
  RETURN QUERY SELECT * FROM groups WHERE id = gid;
  RETURN;
END
$$;

CREATE OR REPLACE FUNCTION get_job_groups_for_origin_v2(op_origin text, op_limit integer) RETURNS SETOF groups
    LANGUAGE sql STABLE
    AS $$
  SELECT *
  FROM groups
  WHERE project_name LIKE (op_origin || '/%')
  ORDER BY created_at DESC
  LIMIT op_limit
$$;

CREATE OR REPLACE FUNCTION get_job_v1(jid bigint) RETURNS SETOF jobs
    LANGUAGE plpgsql STABLE
    AS $$
BEGIN
  RETURN QUERY SELECT * FROM jobs WHERE id = jid;
  RETURN;
END
$$;

CREATE OR REPLACE FUNCTION get_jobs_for_project_v2(p_project_name text, p_limit bigint, p_offset bigint) RETURNS TABLE(total_count bigint, id bigint, owner_id bigint, job_state text, created_at timestamp with time zone, build_started_at timestamp with time zone, build_finished_at timestamp with time zone, package_ident text, project_id bigint, project_name text, project_owner_id bigint, project_plan_path text, vcs text, vcs_arguments text[], net_error_msg text, net_error_code integer, archived boolean)
    LANGUAGE sql STABLE
    AS $$
  SELECT COUNT(*) OVER () AS total_count, id, owner_id, job_state, created_at, build_started_at,
  build_finished_at, package_ident, project_id, project_name, project_owner_id, project_plan_path, vcs,
  vcs_arguments, net_error_msg, net_error_code, archived
  FROM jobs
  WHERE project_name = p_project_name
  ORDER BY created_at DESC
  LIMIT p_limit
  OFFSET p_offset;
$$;

CREATE OR REPLACE FUNCTION get_queued_group_v1(pname text) RETURNS SETOF groups
    LANGUAGE sql
    AS $$
  SELECT * FROM groups
  WHERE project_name = pname
  AND group_state = 'Queued'
$$;

CREATE OR REPLACE FUNCTION get_queued_groups_v1() RETURNS SETOF groups
    LANGUAGE sql
    AS $$
  SELECT * FROM groups
  WHERE group_state = 'Queued'
$$;

CREATE OR REPLACE FUNCTION insert_group_v2(root_project text, project_names text[], project_idents text[]) RETURNS SETOF groups
    LANGUAGE sql
    AS $$
  WITH my_group AS (
          INSERT INTO groups (project_name, group_state)
          VALUES (root_project, 'Queued') RETURNING *
      ), my_project AS (
          INSERT INTO group_projects (owner_id, project_name, project_ident, project_state)
          SELECT g.id, project_info.name, project_info.ident, 'NotStarted'
          FROM my_group AS g, unnest(project_names, project_idents) AS project_info(name, ident)
      )
  SELECT * FROM my_group;
$$;

CREATE OR REPLACE FUNCTION insert_job_v2(p_owner_id bigint, p_project_id bigint, p_project_name text, p_project_owner_id bigint, p_project_plan_path text, p_vcs text, p_vcs_arguments text[], p_channel text) RETURNS SETOF jobs
    LANGUAGE sql
    AS $$
      INSERT INTO jobs (owner_id, job_state, project_id, project_name, project_owner_id, project_plan_path, vcs, vcs_arguments, channel)
      VALUES (p_owner_id, 'Pending', p_project_id, p_project_name, p_project_owner_id, p_project_plan_path, p_vcs, p_vcs_arguments, p_channel)
      RETURNING *;
$$;

CREATE OR REPLACE FUNCTION mark_as_archived_v1(p_job_id bigint) RETURNS void
    LANGUAGE sql
    AS $$
  UPDATE jobs
  SET archived = TRUE
  WHERE id = p_job_id;
$$;

CREATE OR REPLACE FUNCTION next_pending_job_v1(p_worker text) RETURNS SETOF jobs
    LANGUAGE plpgsql
    AS $$
DECLARE
    r jobs % rowtype;
BEGIN
    FOR r IN
        SELECT * FROM jobs
        WHERE job_state = 'Pending'
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

CREATE OR REPLACE FUNCTION pending_groups_v1(integer) RETURNS SETOF groups
    LANGUAGE plpgsql
    AS $_$
DECLARE
    r groups % rowtype;
BEGIN
    FOR r IN
        SELECT * FROM groups
        WHERE group_state = 'Pending'
        ORDER BY created_at ASC
        FOR UPDATE SKIP LOCKED
        LIMIT $1
    LOOP
        UPDATE groups SET group_state='Dispatching', updated_at=now() WHERE id=r.id RETURNING * INTO r;
        RETURN NEXT r;
    END LOOP;
  RETURN;
END
$_$;

CREATE OR REPLACE FUNCTION pending_jobs_v1(integer) RETURNS SETOF jobs
    LANGUAGE plpgsql
    AS $_$
DECLARE
  r jobs % rowtype;
BEGIN
  FOR r IN
    SELECT * FROM jobs
    WHERE job_state = 'Pending'
    ORDER BY created_at ASC
    FOR UPDATE SKIP LOCKED
    LIMIT $1
  LOOP
    UPDATE jobs SET job_state='Dispatched', scheduler_sync=false, updated_at=now() WHERE id=r.id RETURNING * INTO r;
    RETURN NEXT r;
  END LOOP;
  RETURN;
END
$_$;

CREATE OR REPLACE FUNCTION set_group_project_name_state_v1(gid bigint, pname text, state text) RETURNS void
    LANGUAGE plpgsql
    AS $$
  BEGIN
    UPDATE group_projects SET project_state=state, updated_at=now() WHERE owner_id=gid AND project_name=pname;
  END
$$;

CREATE OR REPLACE FUNCTION set_group_project_state_ident_v1(pid bigint, jid bigint, state text, ident text) RETURNS void
    LANGUAGE sql
    AS $$
  UPDATE group_projects SET project_state=state, job_id=jid, project_ident=ident, updated_at=now() WHERE id=pid;
$$;

CREATE OR REPLACE FUNCTION set_group_project_state_v1(pid bigint, jid bigint, state text) RETURNS void
    LANGUAGE plpgsql
    AS $$
  BEGIN
    UPDATE group_projects SET project_state=state, job_id=jid, updated_at=now() WHERE id=pid;
  END
$$;

CREATE OR REPLACE FUNCTION set_group_state_v1(gid bigint, gstate text) RETURNS void
    LANGUAGE plpgsql
    AS $$
  BEGIN
      UPDATE groups SET group_state=gstate, updated_at=now() WHERE id=gid;
  END
$$;

CREATE OR REPLACE FUNCTION set_jobs_sync_v2(in_job_id bigint) RETURNS void
    LANGUAGE sql
    AS $$
  UPDATE jobs SET scheduler_sync = true, sync_count = sync_count-1 WHERE id = in_job_id;
$$;

CREATE OR REPLACE FUNCTION sync_jobs_v2() RETURNS SETOF jobs
    LANGUAGE sql STABLE
    AS $$
  SELECT * FROM jobs WHERE (scheduler_sync = false) OR (sync_count > 0);
$$;

CREATE OR REPLACE FUNCTION update_job_v3(p_job_id bigint, p_state text, p_build_started_at timestamp with time zone, p_build_finished_at timestamp with time zone, p_package_ident text, p_err_code integer, p_err_msg text) RETURNS void
    LANGUAGE sql
    AS $$
  UPDATE jobs
  SET job_state = p_state,
      scheduler_sync = false,
      sync_count = sync_count + 1,
      updated_at = now(),
      build_started_at = p_build_started_at,
      build_finished_at = p_build_finished_at,
      package_ident = p_package_ident,
      net_error_code = p_err_code,
      net_error_msg = p_err_msg
  WHERE id = p_job_id;
$$;

CREATE OR REPLACE FUNCTION upsert_busy_worker_v1(in_ident text, in_job_id bigint, in_quarantined boolean) RETURNS SETOF busy_workers
    LANGUAGE plpgsql
    AS $$
  BEGIN
    RETURN QUERY INSERT INTO busy_workers (ident, job_id, quarantined)
    VALUES (in_ident, in_job_id, in_quarantined)
    ON CONFLICT(ident, job_id)
    DO UPDATE SET quarantined=in_quarantined RETURNING *;
    RETURN;
  END
$$;

CREATE INDEX IF NOT EXISTS pending_groups_index_v1 ON groups(created_at) WHERE (group_state = 'Pending');
CREATE INDEX IF NOT EXISTS pending_jobs_index_v1 ON jobs(created_at) WHERE (job_state = 'Pending');
CREATE INDEX IF NOT EXISTS queued_groups_index_v1 ON groups(created_at) WHERE (group_state = 'Queued');
