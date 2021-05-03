--
-- Cleanup all the SQL functions which no longer in use after converting them to diesel
--

--
-- See http://diesel.rs/guides/all-about-inserts/
-- https://github.com/diesel-rs/diesel/blob/master/CHANGELOG.md#added-21
--
SELECT diesel_manage_updated_at('groups');
SELECT diesel_manage_updated_at('group_projects');
SELECT diesel_manage_updated_at('jobs');
SELECT diesel_manage_updated_at('busy_workers');

--
-- Below are the functions which are no longer in use after converting them to diesel (components/builder-jobsrv/src/data_store.rs)
--

DROP FUNCTION IF EXISTS insert_job_v3(bigint, bigint, text, bigint, text, text, text[], text, text);
DROP FUNCTION IF EXISTS get_job_v1(bigint);
DROP FUNCTION IF EXISTS get_cancel_pending_jobs_v1();
DROP FUNCTION IF EXISTS get_dispatched_jobs_v1();
DROP FUNCTION IF EXISTS count_jobs_v1(text);
DROP FUNCTION IF EXISTS update_job_v3(bigint, text, timestamp with time zone, timestamp with time zone, text, integer, text);
DROP FUNCTION IF EXISTS mark_as_archived_v1(bigint);
DROP FUNCTION IF EXISTS cancel_group_v1(bigint);
DROP FUNCTION IF EXISTS add_audit_jobs_entry_v1(bigint, smallint, smallint, bigint, text);
DROP FUNCTION IF EXISTS get_job_groups_for_origin_v2(text, integer);
DROP FUNCTION IF EXISTS get_group_v1(bigint);
DROP FUNCTION IF EXISTS get_group_projects_for_group_v1(bigint);
DROP FUNCTION IF EXISTS set_group_state_v1(bigint, text);
DROP FUNCTION IF EXISTS set_group_project_name_state_v1(bigint, text, text);
DROP FUNCTION IF EXISTS find_group_project_v1(bigint, text);
DROP FUNCTION IF EXISTS set_group_project_state_ident_v1(bigint, bigint, text, text);
DROP FUNCTION IF EXISTS set_group_project_state_v1(bigint, bigint, text);
DROP FUNCTION IF EXISTS sync_jobs_v2();
DROP FUNCTION IF EXISTS set_jobs_sync_v2(bigint);

--
-- Below are some more functions which are not in use (taken from eeyun/pg_derives_v2)
-- 

DROP FUNCTION IF EXISTS abort_group_v1(bigint);
DROP FUNCTION IF EXISTS check_active_group_v1(text);
DROP FUNCTION IF EXISTS count_group_projects_v2(text);
DROP FUNCTION IF EXISTS count_origin_packages_v1(text);
DROP FUNCTION IF EXISTS count_unique_origin_packages_v1(text);
DROP FUNCTION IF EXISTS delete_busy_worker_v1(text, bigint);
DROP FUNCTION IF EXISTS get_busy_workers_v1();
DROP FUNCTION IF EXISTS get_jobs_for_project_v2(text, bigint, bigint);
DROP FUNCTION IF EXISTS get_queued_group_v1(text);
DROP FUNCTION IF EXISTS get_queued_groups_v1();
DROP FUNCTION IF EXISTS insert_group_v2(text, text[], text[]);
DROP FUNCTION IF EXISTS insert_job_v2(bigint, bigint, text, bigint, text, text, text[], text);
DROP FUNCTION IF EXISTS next_pending_job_v1(text);
DROP FUNCTION IF EXISTS pending_jobs_v1(integer);
DROP FUNCTION IF EXISTS upsert_busy_worker_v1(text, bigint, boolean);
