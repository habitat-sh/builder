--
-- Cleanups for converting the postgres function based access to diesel
--


--
-- This saves us the trouble of inserting 'now' when we updated
--
-- See http://diesel.rs/guides/all-about-inserts/
--
SELECT diesel_manage_updated_at('groups');
SELECT diesel_manage_updated_at('group_projects');
SELECT diesel_manage_updated_at('jobs');
SELECT diesel_manage_updated_at('busy_workers');

--
-- These functions have been replaced with Diesel native functions
--

-- TODO RECHECK THAT THESE ARE NOT USED
DROP FUNCTION abort_group_v1 ;
DROP FUNCTION cancel_group_v1 ;
DROP FUNCTION check_active_group_v1
DROP FUNCTION count_group_projects_v2 ;
DROP FUNCTION count_origin_packages_v1 ;
DROP FUNCTION count_unique_origin_packages_v1 ;
DROP FUNCTION find_group_project_v1 ;
DROP FUNCTION get_cancel_pending_jobs_v1 ;
DROP FUNCTION get_dispatched_jobs_v1 ;
DROP FUNCTION get_dispatched_jobs_v1 ;
DROP FUNCTION get_group_projects_for_group_v1 ;
DROP FUNCTION get_group_v1 ;
DROP FUNCTION get_job_v1 ;
DROP FUNCTION get_jobs_for_project_v2 ;
DROP FUNCTION get_queued_group_v1 ;
DROP FUNCTION get_queued_groups_v1 ;
DROP FUNCTION insert_group_v2 ;
DROP FUNCTION insert_group_v2 ;
DROP FUNCTION next_pending_job_v1 ;
DROP FUNCTION pending_groups_v1 ;
DROP FUNCTION pending_jobs_v1 ;
DROP FUNCTION set_group_project_name_state_v1 ;
DROP FUNCTION set_group_project_state_ident_v1 ;
DROP FUNCTION set_group_project_state_v1 ;
DROP FUNCTION set_jobs_sync_v2
DROP FUNCTION sync_jobs_v2

-- USED
-- add_audit_jobs_entry_v1 ;
-- delete_busy_worker_v1
-- get_busy_workers_v1
-- get_job_groups_for_origin_v2
-- mark_as_archived_v1
-- set_group_state_v1
-- set_jobs_sync_v2
-- sync_jobs_v2
-- update_job_v3
-- upsert_busy_worker_v1
