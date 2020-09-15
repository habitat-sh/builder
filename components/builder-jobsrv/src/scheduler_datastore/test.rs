#[cfg(test)]
#[cfg(feature = "postgres_scheduler_tests")]
// cargo test --features postgres_scheduler_tests to enable
// from root
// cargo test -p habitat_builder_jobsrv --features=postgres_scheduler_tests
// --manifest-path=components/builder-jobsrv/Cargo.toml
mod test {
    use crate::{data_store::DataStore,
                hab_core::package::PackageTarget};

    use habitat_builder_db::{datastore_test,
                             models::{jobs::{JobExecState,
                                             JobGraphEntry,
                                             NewJobGraphEntry,
                                             UpdateJobGraphEntry},
                                      package::BuilderPackageTarget}};
    use habitat_builder_protocol::message::jobsrv::*;
    use std::str::FromStr;

    lazy_static! {
        pub static ref TARGET_PLATFORM: BuilderPackageTarget =
            BuilderPackageTarget(PackageTarget::from_str("x86_64-linux").unwrap());
    }

    macro_rules! assert_match {
        ($result:expr, $expected:pat) => {
            match ($result) {
                $expected => {}
                x => {
                    panic!("assertion failed: expected {:?}, received {:?}",
                           stringify!($expected),
                           x)
                }
            };
        };
    }

    mod helpers {
        use crate::{data_store::DataStore,
                    hab_core::package::PackageTarget};
        use chrono::{DateTime,
                     Duration,
                     Utc};
        use habitat_builder_db::models::{jobs::{JobExecState,
                                                JobGraphEntry,
                                                NewJobGraphEntry,
                                                UpdateJobGraphEntry},
                                         package::BuilderPackageTarget};
        use habitat_builder_protocol::message::{jobsrv::*,
                                                originsrv::OriginProject};
        use std::{collections::HashMap,
                  str::FromStr};

        pub fn is_recent(time: Option<DateTime<Utc>>, tolerance: isize) -> bool {
            Utc::now() - time.unwrap() < Duration::seconds(tolerance as i64)
        }

        // We expect things to have the same time, but sometimes rounding bites us
        pub fn about_same_time(left: Option<DateTime<Utc>>, right: DateTime<Utc>) -> bool {
            (left.unwrap().timestamp_millis() - right.timestamp_millis()).abs() < 100
        }

        pub fn manifest_data_from_file() -> Vec<(String, String, Vec<String>)> {
            let manifest = include_str!("manifest_data.txt");
            let mut data = Vec::new();
            for line in manifest.lines() {
                let fields: Vec<String> = line.split_whitespace().map(|x| x.to_string()).collect();
                let deps: Vec<String> = fields[2].split(',').map(|x| x.to_string()).collect();
                data.push((fields[0].clone(), fields[1].clone(), deps));
            }
            data
        }

        #[derive(Default, Debug, Clone)]
        pub struct JobStateCounts {
            pub p:  i64,
            pub s:  i64,
            pub e:  i64,
            pub d:  i64,
            pub c:  i64,
            pub jf: i64,
            pub df: i64,
            pub cp: i64,
            pub cc: i64,
        }

        pub fn job_state_count_s(gid: i64, conn: &diesel::pg::PgConnection) -> JobStateCounts {
            let mut j = JobStateCounts::default();
            j.p = JobGraphEntry::count_by_state(gid, JobExecState::Pending, &conn).unwrap();
            j.s = JobGraphEntry::count_by_state(gid, JobExecState::Schedulable, &conn).unwrap();
            j.e = JobGraphEntry::count_by_state(gid, JobExecState::Eligible, &conn).unwrap();
            j.d = JobGraphEntry::count_by_state(gid, JobExecState::Dispatched, &conn).unwrap();
            j.c = JobGraphEntry::count_by_state(gid, JobExecState::Complete, &conn).unwrap();
            j
        }

        pub fn job_state_count(gid: i64,
                               conn: &diesel::pg::PgConnection)
                               -> (i64, i64, i64, i64, i64) {
            let schedulable =
                JobGraphEntry::count_by_state(gid, JobExecState::Schedulable, &conn).unwrap();
            let eligible =
                JobGraphEntry::count_by_state(gid, JobExecState::Eligible, &conn).unwrap();
            let complete =
                JobGraphEntry::count_by_state(gid, JobExecState::Complete, &conn).unwrap();
            let failed =
                JobGraphEntry::count_by_state(gid, JobExecState::JobFailed, &conn).unwrap();
            let dep_failed =
                JobGraphEntry::count_by_state(gid, JobExecState::DependencyFailed, &conn).unwrap();

            (schedulable, eligible, complete, failed, dep_failed)
        }

        pub struct DbHelper {
            group_id: i64,
            target:   BuilderPackageTarget,
            name_map: HashMap<String, i64>,
            id_map:   HashMap<i64, String>,
        }

        impl DbHelper {
            pub fn new(group_id: i64, target: &str) -> Self {
                DbHelper { group_id,
                           target: BuilderPackageTarget(PackageTarget::from_str(target).unwrap()),
                           name_map: HashMap::new(),
                           id_map: HashMap::new() }
            }

            pub fn add(&mut self,
                       conn: &diesel::pg::PgConnection,
                       name: &str,
                       deps: &[&str],
                       job_state: JobExecState)
                       -> i64 {
                let dependencies: Vec<i64> =
                    deps.iter()
                        .map(|d| {
                            *(self.name_map
                                  .get(d.to_owned())
                                  .unwrap_or_else(|| panic!("Dependency {} not found", d)))
                        })
                        .collect();

                let plan_name = name.split('/').take(2).collect::<Vec<&str>>().join("/");

                let entry = NewJobGraphEntry { group_id: self.group_id,
                                               job_state,
                                               plan_ident: &plan_name,
                                               manifest_ident: name,
                                               as_built_ident: None,
                                               dependencies: &dependencies,
                                               waiting_on_count: dependencies.len() as i32,
                                               target_platform: &self.target };

                let job_graph_entry = JobGraphEntry::create(&entry, &conn).unwrap();

                self.name_map.insert(name.to_owned(), job_graph_entry.id);
                self.id_map.insert(job_graph_entry.id, name.to_owned());
                job_graph_entry.id
            }

            pub fn id_by_name(&self, name: &str) -> i64 {
                *(self.name_map
                      .get(name)
                      .unwrap_or_else(|| panic!("No entry for {}", name)))
            }

            pub fn name_by_id(&self, id: i64) -> String {
                self.id_map
                    .get(&id)
                    .unwrap_or_else(|| panic!("No entry for {}", id))
                    .clone()
            }
        }

        pub fn make_simple_graph_helper(group_id: i64,
                                        target_platform: &BuilderPackageTarget,
                                        conn: &diesel::pg::PgConnection)
                                        -> DbHelper {
            let mut h = DbHelper::new(group_id, target_platform);

            h.add(conn, "foo/bar/1.2.3/123", &[], JobExecState::Eligible);
            h.add(conn,
                  "foo/baz/1.2.3/123",
                  &["foo/bar/1.2.3/123"],
                  JobExecState::Schedulable);
            h.add(conn,
                  "foo/ping/1.2.3/123",
                  &["foo/bar/1.2.3/123"],
                  JobExecState::Schedulable);
            h.add(conn,
                  "foo/pong/1.2.3/123",
                  &["foo/baz/1.2.3/123", "foo/ping/1.2.3/123"],
                  JobExecState::Schedulable);

            h
        }

        pub fn make_job_graph_entries(group_id: i64,
                                      job_state: JobExecState,
                                      target_platform: &BuilderPackageTarget,
                                      data: &Vec<(String, String, Vec<String>)>,
                                      conn: &diesel::pg::PgConnection)
                                      -> HashMap<String, JobGraphEntry> {
            let mut jobs: HashMap<String, JobGraphEntry> = HashMap::new();
            for (plan_ident, manifest_ident, deps) in data {
                let dependencies: Vec<i64> = deps.iter()
                                                 .filter_map(|d| jobs.get(d).map(|x| x.id))
                                                 .collect();
                let entry = NewJobGraphEntry { group_id,
                                               job_state,
                                               plan_ident: &plan_ident,
                                               manifest_ident: &manifest_ident,
                                               as_built_ident: None,
                                               dependencies: &dependencies,
                                               waiting_on_count: dependencies.len() as i32,
                                               target_platform };
                let job = JobGraphEntry::create(&entry, &conn).unwrap();
                jobs.insert(manifest_ident.clone(), job);
            }
            jobs
        }
    }

    #[test]
    fn create_job_graph_entry() {
        let target_platform =
            BuilderPackageTarget(PackageTarget::from_str("x86_64-linux").unwrap());
        let ds = datastore_test!(DataStore);
        let conn = ds.get_pool().get_conn().unwrap();
        let slice: [i64; 3] = [1, 2, 3];
        let entry = NewJobGraphEntry { group_id:         0,
                                       job_state:        JobExecState::Pending,
                                       plan_ident:       "foo/bar",
                                       manifest_ident:   "foo/bar/1.2.3/123",
                                       as_built_ident:   None,
                                       dependencies:     &[1, 2, 3],
                                       waiting_on_count: 3,
                                       target_platform:  &target_platform, };

        let job_graph_entry = JobGraphEntry::create(&entry, &conn).unwrap();

        assert_eq!(job_graph_entry.group_id, 0);
        assert_eq!(job_graph_entry.job_state, JobExecState::Pending);
    }

    #[test]
    fn take_next_job_for_target() {
        let target_platform =
            BuilderPackageTarget(PackageTarget::from_str("x86_64-linux").unwrap());

        let ds = datastore_test!(DataStore);
        let conn = ds.get_pool().get_conn().unwrap();

        let mut h = helpers::DbHelper::new(0, "x86_64-linux");
        h.add(&conn, "foo/bar/1.2.3/123", &[], JobExecState::Pending);
        h.add(&conn, "foo/baz/1.2.3/123", &[], JobExecState::Schedulable);
        h.add(&conn, "foo/ping/1.2.3/123", &[], JobExecState::Eligible);

        let mut h_alt = helpers::DbHelper::new(1, "x86_64-windows");
        h_alt.add(&conn, "foo/pong/1.2.3/123", &[], JobExecState::Eligible);

        let job_next = JobGraphEntry::take_next_job_for_target(&target_platform, &conn).unwrap();
        assert!(job_next.is_some());
        assert_eq!(job_next.unwrap().id, h.id_by_name("foo/ping/1.2.3/123"));
        // TODO verify we update the state to 'dispatched'

        let job_next = JobGraphEntry::take_next_job_for_target(&target_platform, &conn).unwrap();
        assert!(job_next.is_none());
    }

    #[test]
    fn insert_many_jobs() {
        let target_platform =
            BuilderPackageTarget(PackageTarget::from_str("x86_64-linux").unwrap());
        let ds = datastore_test!(DataStore);
        let conn = ds.get_pool().get_conn().unwrap();

        let manifest = helpers::manifest_data_from_file();
        for i in 1..50 {
            helpers::make_job_graph_entries(i as i64,
                                            JobExecState::Schedulable,
                                            &target_platform,
                                            &manifest,
                                            &conn);
        }
        // Test some stuff for real here
        // std::thread::sleep(std::time::Duration::from_secs(10000));
    }

    #[test]
    fn count_by_state() {
        let ds = datastore_test!(DataStore);
        let conn = ds.get_pool().get_conn().unwrap();

        let group_id = 1;
        helpers::make_simple_graph_helper(0, &TARGET_PLATFORM, &conn);

        assert_eq!(JobGraphEntry::count_by_state(group_id, JobExecState::Schedulable, &conn).unwrap(),
        0);
        assert_eq!(JobGraphEntry::count_by_state(group_id, JobExecState::Eligible, &conn).unwrap(),
                   0);
        assert_eq!(JobGraphEntry::count_by_state(group_id, JobExecState::Complete, &conn).unwrap(),
                   0);

        helpers::make_simple_graph_helper(group_id, &TARGET_PLATFORM, &conn);

        assert_eq!(JobGraphEntry::count_by_state(group_id, JobExecState::Schedulable, &conn).unwrap(),
                   3);
        assert_eq!(JobGraphEntry::count_by_state(group_id, JobExecState::Eligible, &conn).unwrap(),
                   1);
        assert_eq!(JobGraphEntry::count_by_state(group_id, JobExecState::Complete, &conn).unwrap(),
                   0);
    }

    #[test]
    fn transitive_rdeps_for_id_diamond() {
        let ds = datastore_test!(DataStore);
        let conn = ds.get_pool().get_conn().unwrap();

        let mut h = helpers::make_simple_graph_helper(0, &TARGET_PLATFORM, &conn);

        let rdeps = JobGraphEntry::transitive_rdeps_for_id(0, &conn).unwrap();
        assert_eq!(rdeps.len(), 0);

        let mut rdeps = JobGraphEntry::transitive_rdeps_for_id(1, &conn).unwrap();
        rdeps.sort();

        assert_eq!(rdeps.len(), 3);
        assert_eq!(rdeps, vec![2, 3, 4]);

        let rdeps = JobGraphEntry::transitive_rdeps_for_id(2, &conn).unwrap();
        assert_eq!(rdeps, vec![4]);

        let rdeps = JobGraphEntry::transitive_rdeps_for_id(3, &conn).unwrap();
        assert_eq!(rdeps, vec![4]);

        let rdeps = JobGraphEntry::transitive_rdeps_for_id(4, &conn).unwrap();
        assert_eq!(rdeps.len(), 0);
    }

    #[test]
    fn transitive_deps_for_id_diamond() {
        let ds = datastore_test!(DataStore);
        let conn = ds.get_pool().get_conn().unwrap();

        let mut h = helpers::make_simple_graph_helper(0, &TARGET_PLATFORM, &conn);

        let deps = JobGraphEntry::transitive_deps_for_id(0, &conn).unwrap();
        assert_eq!(deps.len(), 0);

        let deps = JobGraphEntry::transitive_deps_for_id(1, &conn).unwrap();
        assert_eq!(deps.len(), 0);

        let deps = JobGraphEntry::transitive_deps_for_id(2, &conn).unwrap();
        assert_eq!(deps, vec![1]);

        let deps = JobGraphEntry::transitive_deps_for_id(3, &conn).unwrap();
        assert_eq!(deps, vec![1]);

        let mut deps = JobGraphEntry::transitive_deps_for_id(4, &conn).unwrap();
        deps.sort();
        assert_eq!(deps, vec![1, 2, 3]);
    }

    #[test]
    fn mark_job_complete() {
        let ds = datastore_test!(DataStore);
        let conn = ds.get_pool().get_conn().unwrap();

        helpers::make_simple_graph_helper(1, &TARGET_PLATFORM, &conn);
        helpers::make_simple_graph_helper(2, &TARGET_PLATFORM, &conn); // This group should not be scheduled

        // We prefer group 1 while there is work left; then group 2
        let job_next = JobGraphEntry::take_next_job_for_target(&TARGET_PLATFORM, &conn).unwrap();
        assert!(job_next.is_some());
        let job_data = job_next.unwrap();
        assert_eq!(job_data.plan_ident, "foo/bar");
        assert_eq!(job_data.group_id, 1);
        let ready = JobGraphEntry::mark_job_complete(job_data.id, &conn);
        assert_eq!(ready.unwrap(), 2);

        assert_eq!((1, 2, 1, 0, 0), helpers::job_state_count(1, &conn));

        let job_next = JobGraphEntry::take_next_job_for_target(&TARGET_PLATFORM, &conn).unwrap();
        assert!(job_next.is_some());
        let job_data = job_next.unwrap();
        assert_eq!(job_data.group_id, 1);
        let ready = JobGraphEntry::mark_job_complete(job_data.id, &conn);
        assert_eq!(ready.unwrap(), 0);

        assert_eq!((1, 1, 2, 0, 0), helpers::job_state_count(1, &conn));

        let job_next = JobGraphEntry::take_next_job_for_target(&TARGET_PLATFORM, &conn).unwrap();
        assert!(job_next.is_some());
        let job_data = job_next.unwrap();
        assert_eq!(job_data.group_id, 1);
        let ready = JobGraphEntry::mark_job_complete(job_data.id, &conn);
        assert_eq!(ready.unwrap(), 1);

        assert_eq!((0, 1, 3, 0, 0), helpers::job_state_count(1, &conn));

        let job_next = JobGraphEntry::take_next_job_for_target(&TARGET_PLATFORM, &conn).unwrap();
        assert!(job_next.is_some());
        let job_data = job_next.unwrap();
        assert_eq!(job_data.group_id, 1);
        let ready = JobGraphEntry::mark_job_complete(job_data.id, &conn);
        assert_eq!(ready.unwrap(), 0);

        assert_eq!((0, 0, 4, 0, 0), helpers::job_state_count(1, &conn));

        let job_next = JobGraphEntry::take_next_job_for_target(&TARGET_PLATFORM, &conn).unwrap();
        assert!(job_next.is_some());
        let job_data = job_next.unwrap();
        assert_eq!(job_data.group_id, 2);
    }

    #[test]
    fn mark_job_complete_interleaved() {
        let ds = datastore_test!(DataStore);
        let conn = ds.get_pool().get_conn().unwrap();

        helpers::make_simple_graph_helper(1, &TARGET_PLATFORM, &conn);
        helpers::make_simple_graph_helper(2, &TARGET_PLATFORM, &conn); // This group should not be scheduled

        let job_next = JobGraphEntry::take_next_job_for_target(&TARGET_PLATFORM, &conn).unwrap();
        assert!(job_next.is_some());
        let job_data = job_next.unwrap();
        assert_eq!(job_data.plan_ident, "foo/bar");
        assert_eq!(job_data.group_id, 1);
        let ready = JobGraphEntry::mark_job_complete(job_data.id, &conn);
        assert_eq!(ready.unwrap(), 2);

        assert_eq!((1, 2, 1, 0, 0), helpers::job_state_count(1, &conn));

        assert_match!(helpers::job_state_count_s(1, &conn),
                      helpers::JobStateCounts { p: 0, s: 1, e: 2, d: 0, c: 1, .. });

        assert_eq!((3, 1, 0, 0, 0), helpers::job_state_count(2, &conn));

        // Get another job from group 1
        let job_a = JobGraphEntry::take_next_job_for_target(&TARGET_PLATFORM, &conn).unwrap()
                                                                                    .unwrap();
        assert_eq!(job_a.group_id, 1);
        assert_eq!((1, 1, 1, 0, 0), helpers::job_state_count(1, &conn));
        assert_eq!((3, 1, 0, 0, 0), helpers::job_state_count(2, &conn));

        // Get another job, expect group 1
        let job_b = JobGraphEntry::take_next_job_for_target(&TARGET_PLATFORM, &conn).unwrap()
                                                                                    .unwrap();
        assert_eq!(job_b.group_id, 1);
        assert_eq!((1, 0, 1, 0, 0), helpers::job_state_count(1, &conn));
        assert_eq!((3, 1, 0, 0, 0), helpers::job_state_count(2, &conn));

        // There are no more group one jobs, so expect group 2
        let job_c = JobGraphEntry::take_next_job_for_target(&TARGET_PLATFORM, &conn).unwrap()
                                                                                    .unwrap();

        assert_eq!(job_c.group_id, 2);
        assert_eq!((1, 0, 1, 0, 0), helpers::job_state_count(1, &conn));
        assert_eq!((3, 0, 0, 0, 0), helpers::job_state_count(2, &conn));

        let ready = JobGraphEntry::mark_job_complete(job_a.id, &conn);
        assert_eq!(ready.unwrap(), 0);
        assert_eq!((1, 0, 2, 0, 0), helpers::job_state_count(1, &conn));
        assert_eq!((3, 0, 0, 0, 0), helpers::job_state_count(2, &conn));

        let ready = JobGraphEntry::mark_job_complete(job_b.id, &conn);
        assert_eq!(ready.unwrap(), 1);
        assert_eq!((0, 1, 3, 0, 0), helpers::job_state_count(1, &conn));
        assert_eq!((3, 0, 0, 0, 0), helpers::job_state_count(2, &conn));

        let ready = JobGraphEntry::mark_job_complete(job_c.id, &conn);
        assert_eq!(ready.unwrap(), 2);
        assert_eq!((0, 1, 3, 0, 0), helpers::job_state_count(1, &conn));
        assert_eq!((1, 2, 1, 0, 0), helpers::job_state_count(2, &conn));

        let job_next = JobGraphEntry::take_next_job_for_target(&TARGET_PLATFORM, &conn).unwrap()
                                                                                       .unwrap();
        assert_eq!(job_next.group_id, 1);
        let ready = JobGraphEntry::mark_job_complete(job_next.id, &conn);
        assert_eq!(ready.unwrap(), 0);
        assert_eq!((0, 0, 4, 0, 0), helpers::job_state_count(1, &conn));
        assert_eq!((1, 2, 1, 0, 0), helpers::job_state_count(2, &conn));
    }
}
