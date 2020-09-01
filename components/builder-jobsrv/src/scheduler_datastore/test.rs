#[cfg(test)]
#[cfg(feature = "postgres_scheduler_tests")]
// cargo test --features postgres_scheduler_tests to enable
// from root
// cargo test -p habitat_builder_jobsrv --features=postgres_scheduler_tests
// --manifest-path=components/builder-jobsrv/Cargo.toml
mod test {
    use crate::{data_store::DataStore,
                hab_core::package::PackageTarget};
    use chrono::{DateTime,
                 Duration,
                 Utc};

    use habitat_builder_db::{datastore_test,
                             models::{jobs::{JobExecState,
                                             JobGraphEntry,
                                             NewJobGraphEntry,
                                             UpdateJobGraphEntry},
                                      package::BuilderPackageTarget}};
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

        pub fn is_recent(time: Option<DateTime<Utc>>, tolerance: isize) -> bool {
            Utc::now() - time.unwrap() < Duration::seconds(tolerance as i64)
        }

        // We expect things to have the same time, but sometimes rounding bites us
        pub fn about_same_time(left: Option<DateTime<Utc>>, right: DateTime<Utc>) -> bool {
            (left.unwrap().timestamp_millis() - right.timestamp_millis()).abs() < 100
        }
    }

    #[test]
    fn create_job_graph_entry() {
        let target_platform =
            BuilderPackageTarget(PackageTarget::from_str("x86_64-linux").unwrap());
        let ds = datastore_test!(DataStore);
        let conn = ds.get_pool().get_conn().unwrap();
        let slice: [i64; 3] = [1, 2, 3];
        let entry = NewJobGraphEntry { group_id:        0,
                                       job_state:       JobExecState::Pending,
                                       plan_ident:      "foo/bar",
                                       manifest_ident:  "foo/bar/1.2.3/123",
                                       as_built_ident:  None,
                                       dependencies:    &[1, 2, 3],
                                       target_platform: &target_platform, };

        let job_graph_entry = JobGraphEntry::create(&entry, &conn).unwrap();

        assert_eq!(job_graph_entry.group_id, 0);
        assert_eq!(job_graph_entry.job_state, JobExecState::Pending);
    }

    #[test]
    fn take_next_job_for_target() {
        let target_platform =
            BuilderPackageTarget(PackageTarget::from_str("x86_64-linux").unwrap());
        let other_platform =
            BuilderPackageTarget(PackageTarget::from_str("x86_64-windows").unwrap());
        let ds = datastore_test!(DataStore);
        let conn = ds.get_pool().get_conn().unwrap();
        let slice: [i64; 3] = [1, 2, 3];
        let entry = NewJobGraphEntry { group_id:        0,
                                       job_state:       JobExecState::Pending,
                                       plan_ident:      "foo/bar",
                                       manifest_ident:  "foo/bar/1.2.3/123",
                                       as_built_ident:  None,
                                       dependencies:    &[1, 2, 3],
                                       target_platform: &target_platform, };

        let job_graph_entry_1 = JobGraphEntry::create(&entry, &conn).unwrap();

        let entry = NewJobGraphEntry { group_id:        0,
                                       job_state:       JobExecState::Schedulable,
                                       plan_ident:      "foo/baz",
                                       manifest_ident:  "foo/baz/1.2.3/123",
                                       as_built_ident:  None,
                                       dependencies:    &[1, 2, 3],
                                       target_platform: &target_platform, };

        let job_graph_entry_2 = JobGraphEntry::create(&entry, &conn).unwrap();

        let entry = NewJobGraphEntry { group_id:        0,
                                       job_state:       JobExecState::Eligible,
                                       plan_ident:      "foo/ping",
                                       manifest_ident:  "foo/ping/1.2.3/123",
                                       as_built_ident:  None,
                                       dependencies:    &[1, 2, 3],
                                       target_platform: &target_platform, };

        let job_graph_entry_3 = JobGraphEntry::create(&entry, &conn).unwrap();

        let entry = NewJobGraphEntry { group_id:        0,
                                       job_state:       JobExecState::Eligible,
                                       plan_ident:      "foo/pong",
                                       manifest_ident:  "foo/pong/1.2.3/123",
                                       as_built_ident:  None,
                                       dependencies:    &[1, 2, 3],
                                       target_platform: &other_platform, };

        let job_graph_entry_4 = JobGraphEntry::create(&entry, &conn).unwrap();

        let job_next = JobGraphEntry::take_next_job_for_target(&target_platform, &conn).unwrap();
        assert!(job_next.is_some());
        assert_eq!(job_next.unwrap(), job_graph_entry_3.id);
        // TODO verify we update the state to 'dispatched'

        let job_next = JobGraphEntry::take_next_job_for_target(&target_platform, &conn).unwrap();
        assert!(job_next.is_none());
    }
}
