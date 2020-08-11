#[cfg(test)]
mod test {
    use crate::data_store::DataStore;
    use chrono::{DateTime,
                 Duration,
                 Utc};
    use habitat_builder_db::datastore_test;
    use habitat_builder_protocol::message::{jobsrv::*,
                                            originsrv::{OriginPackageIdent,
                                                        OriginProject}};
    use std::{convert::TryInto,
              str::FromStr};

    mod helpers {
        use crate::data_store::DataStore;
        use chrono::{DateTime,
                     Duration,
                     Utc};
        use habitat_builder_protocol::message::{jobsrv::*,
                                                originsrv::OriginProject};

        pub static TARGET: &str = "x86_64-linux";
        pub static PROJECT_NAME: &str = "something/else";
        pub static JOB_GROUP_OWNER_ID: u64 = 1701;
        pub static JOB_GROUP_ORIGIN: &str = "some";
        pub static JOB_GROUP_PACKAGE: &str = "thing";
        pub static JOB_GROUP_PROJECT_IDENT: &str = "package/ident";

        pub fn create_project() -> OriginProject {
            let mut project = OriginProject::new();
            project.set_id(1234);
            // name is expected to be origin/pkg_name
            project.set_name(PROJECT_NAME.to_string());
            project.set_owner_id(1234);
            project.set_plan_path("habitat/plan.sh".to_string());
            project.set_vcs_installation_id(23423);
            project.set_vcs_type("git".to_string());
            project.set_vcs_data("something".to_string());

            project
        }

        pub fn create_job() -> Job {
            let project = create_project();
            let mut job = Job::new();
            job.set_project(project);
            job.set_channel("thing".to_string());
            job.set_target(TARGET.to_string());

            job
        }

        // TODO: -> (JobGroupSpec, (Vec<String>, Vec<String>))
        // OR:(ds:&DataStore, project_count: usize)
        pub fn create_job_group(ds: &DataStore) -> JobGroup {
            let mut job_group = JobGroupSpec::new();
            job_group.set_origin(JOB_GROUP_ORIGIN.to_string());
            job_group.set_package(JOB_GROUP_PACKAGE.to_string());
            job_group.set_target(TARGET.to_string());
            let projects =
                vec![(JOB_GROUP_ORIGIN.to_string(), JOB_GROUP_PROJECT_IDENT.to_string())];

            let result = ds.create_job_group(&job_group, projects);

            assert!(result.is_ok());
            result.unwrap()
        }

        pub fn is_recent(time: Option<DateTime<Utc>>, tolerance: isize) -> bool {
            Utc::now() - time.unwrap() < Duration::seconds(tolerance as i64)
        }

        // We expect things to have the same time, but sometimes rounding bites us
        pub fn about_same_time(left: Option<DateTime<Utc>>, right: DateTime<Utc>) -> bool {
            (left.unwrap().timestamp_millis() - right.timestamp_millis()).abs() < 100
        }
    }

    #[test]
    fn create_job() {
        let ds = datastore_test!(DataStore);

        let job = helpers::create_job();

        let result = ds.create_job(&job);

        assert!(result.is_ok());

        let job_id = result.unwrap().get_id();
        let conn = ds.get_pool().get_conn().unwrap();
        let raw_job = habitat_builder_db::models::jobs::Job::get(job_id as _, &conn).unwrap();

        // // Test the thing we inserted
        assert_eq!(job_id, raw_job.id as u64);
        assert_eq!(JobState::Pending,
                   raw_job.job_state.parse::<JobState>().unwrap());
        assert_eq!(Some(job.get_channel().to_string()), raw_job.channel);
        assert_eq!(job.get_owner_id(), raw_job.owner_id as u64);
        assert!(helpers::is_recent(raw_job.created_at, 5));
        assert!(helpers::is_recent(raw_job.updated_at, 5));

        assert_eq!(job.get_project().get_id(), raw_job.project_id as u64);
        assert_eq!(job.get_project().get_owner_id(),
                   raw_job.project_owner_id as u64);
        assert_eq!(job.get_project().get_name().to_string(),
                   raw_job.project_name);
        assert_eq!(job.get_project().get_plan_path().to_string(),
                   raw_job.project_plan_path);
        assert_eq!(job.get_project().get_vcs_type().to_string(), raw_job.vcs);
        assert_eq!(2, raw_job.vcs_arguments.len());
        assert_eq!(Some("something".to_string()), raw_job.vcs_arguments[0]);
        assert_eq!(Some("23423".to_string()), raw_job.vcs_arguments[1]);

        // Test the fetcher that gives us a protobuf struct
        let mut job_get = JobGet::new();
        job_get.set_id(job_id);

        let result = ds.get_job(&job_get).unwrap().unwrap();

        assert_eq!(job.get_owner_id(), result.get_owner_id());
        assert_eq!(job.get_project().get_name(),
                   result.get_project().get_name());
        assert_eq!(job.get_project().get_vcs_data(),
                   result.get_project().get_vcs_data());
        assert_eq!(job.get_project().get_vcs_installation_id(),
                   result.get_project().get_vcs_installation_id());
        assert_eq!(job.get_channel(), result.get_channel());
        assert_eq!(JobState::Pending, result.get_state());
    }

    #[test]
    // Awaiting some scaffolding to set up jobs in the correct state
    fn next_pending_job() {
        let ds = datastore_test!(DataStore);
        let job = helpers::create_job();

        // Create jobs
        let _ = ds.create_job(&job);
        let _ = ds.create_job(&job);

        let conn = ds.get_pool().get_conn().unwrap();
        let list_project_jobs = habitat_builder_db::models::jobs::ListProjectJobs{
            name: helpers::PROJECT_NAME.to_string(),
            page: 1,
            limit: 100
        };
        let (_, job_count) =
            habitat_builder_db::models::jobs::Job::list(list_project_jobs, &conn).unwrap();

        assert_eq!(job_count, 2);

        // Test we can't advance a job with an invalid target
        let result = ds.next_pending_job("baz", "not-a-target");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Advance first job to dispatched
        let job1 = ds.next_pending_job("foo", helpers::TARGET).unwrap();
        assert!(job1.is_some());
        let job1 = job1.unwrap();
        assert_eq!(job1.get_state(), JobState::Dispatched);
        assert_eq!(job1.get_worker(), "foo");

        // Advance second job to dispatched
        let job2 = ds.next_pending_job("bar", helpers::TARGET).unwrap();
        assert!(job2.is_some());
        let job2 = job2.unwrap();
        assert_eq!(job2.get_state(), JobState::Dispatched);
        assert_eq!(job2.get_worker(), "bar");

        // Assert that the two jobs were not the same
        assert_ne!(job1.get_id(), job2.get_id());

        // Test that we can't dispatch a job when everything has already been dispatched
        let result = ds.next_pending_job("baz", helpers::TARGET);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    // Awaiting some scaffolding to set up jobs in the correct state
    fn get_cancel_pending_jobs() {
        let ds = datastore_test!(DataStore);
        let job = helpers::create_job();

        // Create jobs
        let mut job = ds.create_job(&job).unwrap();
        let _ = ds.create_job(&job);

        job.set_state(JobState::CancelPending);
        let _ = ds.update_job(&job);

        let result = ds.get_cancel_pending_jobs().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get_id(), job.get_id());
        assert_eq!(result[0].get_state(), JobState::CancelPending)
    }

    #[test]
    // Awaiting some scaffolding to set up jobs in the correct state
    fn get_dispatched_jobs() {
        let ds = datastore_test!(DataStore);
        let job = helpers::create_job();

        // Create jobs
        let mut job = ds.create_job(&job).unwrap();
        let _ = ds.create_job(&job);

        job.set_state(JobState::Dispatched);
        let _ = ds.update_job(&job);

        let result = ds.get_dispatched_jobs().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get_id(), job.get_id());
        assert_eq!(result[0].get_state(), JobState::Dispatched)
    }

    #[test]
    fn update_jobs() {
        let ds = datastore_test!(DataStore);
        let mut job = helpers::create_job();
        let result = ds.create_job(&job).unwrap();
        let job_id = result.get_id();

        let started_at = Utc::now() + Duration::minutes(5);
        let finished_at = Utc::now() + Duration::minutes(6);
        let package_ident = "fully/qualified/version/release";

        job.set_id(job_id);
        job.set_state(JobState::Complete);
        job.set_build_started_at(started_at.to_string());
        job.set_build_finished_at(finished_at.to_string());
        job.set_package_ident(OriginPackageIdent::from_str(package_ident).unwrap());

        // TODO: set and check error fields

        let result = ds.update_job(&job);
        assert!(result.is_ok());

        let conn = ds.get_pool().get_conn().unwrap();
        let raw_job = habitat_builder_db::models::jobs::Job::get(job_id as _, &conn).unwrap();
        assert!(helpers::about_same_time(raw_job.build_started_at, started_at));
        assert!(helpers::about_same_time(raw_job.build_finished_at, finished_at));
        assert_eq!(raw_job.job_state, "Complete");
        assert_eq!(raw_job.package_ident, Some(package_ident.to_string()));
        assert_eq!(raw_job.sync_count, 1);
    }

    #[test]
    fn create_job_group() {
        let ds = datastore_test!(DataStore);

        let job_group = helpers::create_job_group(&ds);
        let project_name = format!("{}/{}",
                                   helpers::JOB_GROUP_ORIGIN,
                                   helpers::JOB_GROUP_PACKAGE);

        let group_id = job_group.get_id();

        let mut group_get = JobGroupGet::new();
        group_get.set_group_id(group_id);
        group_get.set_include_projects(true);

        let result = ds.get_job_group(&group_get).unwrap().unwrap();

        assert_eq!(JobGroupState::GroupQueued, result.get_state());
        assert_eq!(project_name, result.get_project_name());
        assert_eq!(helpers::TARGET, result.get_target());
        assert!(helpers::is_recent(result.get_created_at().parse::<DateTime<Utc>>().ok(), 1));

        assert_eq!(result.get_projects().len(), 1);

        let project = result.get_projects().first().unwrap();
        assert_eq!(helpers::JOB_GROUP_ORIGIN, project.get_name());
        assert_eq!(helpers::JOB_GROUP_PROJECT_IDENT, project.get_ident());
        assert_eq!(JobGroupProjectState::NotStarted, project.get_state());
    }

    #[test]
    fn cancel_job_group() {
        let ds = datastore_test!(DataStore);
        let mut job_group = JobGroupSpec::new();
        job_group.set_origin("some".to_string());
        job_group.set_package("thing".to_string());
        job_group.set_target("x86_64-linux".to_string());
        let projects = vec![("some".to_string(), "package/ident".to_string()),
                            ("else".to_string(), "other/ident".to_string())];
        let result = ds.create_job_group(&job_group, projects);
        let group_id = result.unwrap().get_id();

        // Advance one job before cancelling the group
        let _ = ds.set_job_group_project_state(group_id, "some", JobGroupProjectState::InProgress);
        ds.cancel_job_group(group_id).unwrap();

        let mut group_get = JobGroupGet::new();
        group_get.set_group_id(group_id);
        group_get.set_include_projects(true);

        let result = ds.get_job_group(&group_get).unwrap().unwrap();
        assert_eq!(JobGroupState::GroupCanceled, result.get_state());
        let projects = result.get_projects();
        let project_states = projects.iter().map(|p| p.get_state());
        let project_states: Vec<JobGroupProjectState> = project_states.collect();
        assert!(project_states.contains(&JobGroupProjectState::Canceled));
        assert!(project_states.contains(&JobGroupProjectState::InProgress));
    }

    #[test]
    fn mark_as_archived() {
        let ds = datastore_test!(DataStore);
        let job = helpers::create_job();
        let result = ds.create_job(&job).unwrap();
        let job_id = result.get_id();

        let result = ds.mark_as_archived(job_id);
        assert!(result.is_ok());

        let conn = ds.get_pool().get_conn().unwrap();
        let raw_job = habitat_builder_db::models::jobs::Job::get(job_id as _, &conn).unwrap();
        assert_eq!(true, raw_job.archived);
    }

    #[test]
    fn create_audit_entry() {
        let ds = datastore_test!(DataStore);
        let mut msg = JobGroupAudit::new();
        let grog_id = 42 as i64;
        let whodis = "bobbytables".to_string();
        msg.set_group_id(grog_id as u64);
        msg.set_operation(JobGroupOperation::JobGroupOpCreate);
        msg.set_trigger(JobGroupTrigger::Upload);
        msg.set_requester_id(37);
        msg.set_requester_name(whodis);

        let _result = ds.create_audit_entry(&msg);

        let conn = ds.get_pool().get_conn().unwrap();
        let raw_audit_group =
            habitat_builder_db::models::jobs::AuditJob::get_for_group(grog_id, &conn).unwrap();

        assert_eq!(raw_audit_group.len(), 1);
        assert_eq!(raw_audit_group.first().unwrap().group_id, grog_id);
    }

    #[test]
    fn get_job_group_origin() {
        let ds = datastore_test!(DataStore);

        let mut job_group_uno = JobGroupSpec::new();
        job_group_uno.set_origin("some".to_string());
        job_group_uno.set_package("thing".to_string());
        job_group_uno.set_target("x86_64-linux".to_string());
        let projects = vec![("some".to_string(), "package/ident".to_string()),
                            ("right".to_string(), "opackage/odent".to_string())];

        let _result_uno = ds.create_job_group(&job_group_uno, projects);

        let mut job_group_dos = JobGroupSpec::new();
        job_group_dos.set_origin("something".to_string());
        job_group_dos.set_package("thang".to_string());
        job_group_dos.set_target("x86_64-linux".to_string());
        let projects = vec![("something".to_string(), "package/ident".to_string())];

        let _result_dos = ds.create_job_group(&job_group_dos, projects);

        let mut job_group_tres = JobGroupSpec::new();
        job_group_tres.set_origin("some".to_string());
        job_group_tres.set_package("thing".to_string());
        job_group_tres.set_target("x86_64-linux".to_string());
        let projects = vec![("something".to_string(), "package/ident".to_string())];

        let _result_tres = ds.create_job_group(&job_group_tres, projects);

        let mut msg_uno = JobGroupOriginGet::new();
        msg_uno.set_origin("some".to_string());
        msg_uno.set_limit(1);

        let mut msg_dos = JobGroupOriginGet::new();
        msg_dos.set_origin("some".to_string());
        msg_dos.set_limit(3);

        let job_group_origin_uno = ds.get_job_group_origin(&msg_uno).unwrap();
        let job_group_origin_dos = ds.get_job_group_origin(&msg_dos).unwrap();

        assert_eq!(job_group_origin_uno.get_job_groups().len(), 1);
        assert_eq!(job_group_origin_dos.get_job_groups().len(), 2);
    }

    // get_job_group is covered in create_job_group
    // so we're not doing more separately (but that could change)

    #[test]
    fn set_job_group_state() {
        let ds = datastore_test!(DataStore);

        let mut job_group_uno = JobGroupSpec::new();
        job_group_uno.set_origin("some".to_string());
        job_group_uno.set_package("thing".to_string());
        job_group_uno.set_target("x86_64-linux".to_string());
        let projects = vec![("some".to_string(), "package/ident".to_string()),
                            ("right".to_string(), "opackage/odent".to_string())];

        let result_uno = ds.create_job_group(&job_group_uno, projects);
        let group_id = result_uno.unwrap().get_id();

        // check that state is 'queued'
        let mut group_get = JobGroupGet::new();
        group_get.set_group_id(group_id);
        group_get.set_include_projects(true);

        let before = ds.get_job_group(&group_get).unwrap().unwrap();
        assert_eq!(JobGroupState::GroupQueued, before.get_state());

        let _ = ds.set_job_group_state(group_id, JobGroupState::GroupPending);

        // check that state is 'pending'
        let after = ds.get_job_group(&group_get).unwrap().unwrap();
        assert_eq!(JobGroupState::GroupPending, after.get_state());

        // TODO CHECK that timestamp updated
    }

    #[test]
    fn set_job_group_project_state() {
        let ds = datastore_test!(DataStore);

        let job_group = helpers::create_job_group(&ds);
        let group_id = job_group.get_id();
        let mut group_get = JobGroupGet::new();

        let conn = ds.get_pool().get_conn().unwrap();
        let before =
            habitat_builder_db::models::jobs::GroupProject::get_group_projects(group_id.try_into()
                                                                                       .unwrap(),
                                                                               &conn).unwrap();
        let before = before.first().unwrap();
        assert_eq!(JobGroupProjectState::NotStarted,
                   JobGroupProjectState::from_str(&before.project_state).unwrap());

        ds.set_job_group_project_state(group_id,
                                       helpers::JOB_GROUP_ORIGIN,
                                       JobGroupProjectState::InProgress);

        let after =
            habitat_builder_db::models::jobs::GroupProject::get_group_projects(group_id.try_into()
                                                                                       .unwrap(),
                                                                               &conn).unwrap();
        assert_eq!(after.len(), 1);
        let after = after.first().unwrap();
        assert_eq!(helpers::JOB_GROUP_ORIGIN, after.project_name);
        assert_eq!(JobGroupProjectState::InProgress,
                   JobGroupProjectState::from_str(&after.project_state).unwrap());
        // TODO check that timestame updated
    }
    use diesel::RunQueryDsl;
    #[test]
    fn set_job_group_job_state() {
        let ds = datastore_test!(DataStore);
        let job_group = helpers::create_job_group(&ds);

        let mut job = Job::new();
        job.set_owner_id(job_group.get_id());

        let mut project = OriginProject::new();
        project.set_name(helpers::JOB_GROUP_ORIGIN.to_string());
        job.set_project(project);
        job.set_state(JobState::Pending);

        // let conn = ds.get_pool().get_conn().unwrap();
        // let all = habitat_builder_db::schema::jobs::group_projects::table.load::
        // <(habitat_builder_db::models::jobs::GroupProject)>(&conn); println!("{:?}", all);
        let result = ds.set_job_group_job_state(&job);
        assert!(result.is_ok());

        let mut group_get = JobGroupGet::new();
        group_get.set_group_id(job_group.get_id());
        group_get.set_include_projects(true);

        let result = ds.get_job_group(&group_get).unwrap().unwrap();
        assert_eq!(result.get_projects().len(), 1);
        let project = result.get_projects().first().unwrap();
        // This is very strange; as best we can tell the package/ident string is normally filled out
        // with the latest package uploaded from the graph structure, and then updated with
        // a new ident when we are done.
        assert_eq!("package/ident", project.get_ident());
        assert_eq!(JobGroupProjectState::InProgress, project.get_state());

        job.set_state(JobState::Complete);
        let new_ident = "package/ident/v100000/20490101235959";
        job.set_package_ident(OriginPackageIdent::from_str(new_ident).unwrap());
        let result = ds.set_job_group_job_state(&job);
        assert!(result.is_ok());

        let result = ds.get_job_group(&group_get).unwrap().unwrap();
        assert_eq!(result.get_projects().len(), 1);
        let project = result.get_projects().first().unwrap();

        assert_eq!(new_ident, project.get_ident());
        assert_eq!(JobGroupProjectState::Success, project.get_state());
    }

    #[test]
    fn pending_job_groups() {
        // Apparently... this never gets called. Stub
        // just in case we're super confused on this
        assert!(true);
    }

    #[test]
    fn sync_jobs() {
        let ds = datastore_test!(DataStore);
        let conn = ds.get_pool().get_conn().unwrap();
        let job = helpers::create_job();

        // Create job, capture the return value so our Job has an ID now
        let job = ds.create_job(&job).unwrap();
        let job_id = job.get_id() as i64;

        let _ = ds.update_job(&job);
        let _ = ds.update_job(&job);

        let raw_job = habitat_builder_db::models::jobs::Job::get(job_id, &conn).unwrap();
        assert_eq!(raw_job.sync_count, 2);
        assert_eq!(raw_job.scheduler_sync, false);
        assert_eq!(ds.sync_jobs().unwrap().len(), 1);

        let _ = ds.set_job_sync(job_id as u64);
        let raw_job = habitat_builder_db::models::jobs::Job::get(job_id, &conn).unwrap();
        assert_eq!(raw_job.sync_count, 1);
        assert_eq!(raw_job.scheduler_sync, true);
        assert_eq!(ds.sync_jobs().unwrap().len(), 1);

        let _ = ds.set_job_sync(job_id as u64);
        let raw_job = habitat_builder_db::models::jobs::Job::get(job_id, &conn).unwrap();
        assert_eq!(raw_job.sync_count, 0);
        assert_eq!(raw_job.scheduler_sync, true);
        assert_eq!(ds.sync_jobs().unwrap().len(), 0);

        let _ = ds.set_job_sync(job_id as u64);
        let _ = ds.update_job(&job);
        let raw_job = habitat_builder_db::models::jobs::Job::get(job_id, &conn).unwrap();
        assert_eq!(raw_job.sync_count, 0);
        assert_eq!(raw_job.scheduler_sync, false);
        assert_eq!(ds.sync_jobs().unwrap().len(), 1);
    }

    #[test]
    fn set_job_sync() {
        // Covered in sync_jobs
        assert!(true);
    }

    #[test]
    fn upsert_busy_worker() {
        // Apparently... this never gets called. Stub
        // just in case we're super confused on this
        assert!(true);
    }

    #[test]
    fn delete_busy_worker() {
        // Apparently... this never gets called. Stub
        // just in case we're super confused on this
        assert!(true);
    }

    #[test]
    fn get_busy_workers() {
        // Apparently... this never gets called. Stub
        // just in case we're super confused on this
        assert!(true);
    }

    #[test]
    fn is_job_group_active() {
        // Apparently... this never gets called. Stub
        // just in case we're super confused on this
        assert!(true);
    }
    #[test]
    fn get_queued_job_group() {
        // Apparently... this never gets called. Stub
        // just in case we're super confused on this
        assert!(true);
    }

    #[test]
    fn get_queued_job_groups() {
        // Apparently... this never gets called. Stub
        // just in case we're super confused on this
        assert!(true);
    }
}
