#[cfg(test)]
#[cfg(feature = "postgres_tests")]
// cargo test --features postgres_tests to enable
// from root
// cargo test -p habitat_builder_jobsrv --features=postgres_tests
// --manifest-path=components/builder-jobsrv/Cargo.toml
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

        pub fn is_recent(time: Option<DateTime<Utc>>, tolerance: isize) -> bool {
            Utc::now() - time.unwrap() < Duration::seconds(tolerance as i64)
        }

        // We expect things to have the same time, but sometimes rounding bites us
        pub fn about_same_time(left: Option<DateTime<Utc>>, right: DateTime<Utc>) -> bool {
            (left.unwrap().timestamp_millis() - right.timestamp_millis()).abs() < 100
        }
    }

    #[test]
    fn create_job_graph_entry() { let ds = datastore_test!(DataStore); }
}
