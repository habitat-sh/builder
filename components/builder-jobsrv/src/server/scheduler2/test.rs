#[cfg(test)]
#[cfg(feature = "postgres_scheduler_tests")]
// cargo test --features postgres_scheduler_tests to enable
// from root
// cargo test -p habitat_builder_jobsrv --features=postgres_scheduler_tests
// --manifest-path=components/builder-jobsrv/Cargo.toml
mod test {

    use super::super::*;

    use crate::{db::models::{jobs::{JobExecState,
                                    JobGraphEntry,
                                    NewJobGraphEntry},
                             package::BuilderPackageTarget},
                error::Result,
                scheduler_datastore::{DummySchedulerDataStore,
                                      DummySchedulerDataStoreCall,
                                      DummySchedulerDataStoreResult,
                                      JobGraphId,
                                      SchedulerDataStore,
                                      SchedulerDataStoreDb,
                                      WorkerId}};

    use crate::hab_core::package::PackageTarget;

    use std::str::FromStr;

    use chrono::{TimeZone,
                 Utc};

    use lazy_static::lazy_static;

    lazy_static! {
        pub static ref TARGET_PLATFORM: BuilderPackageTarget =
            BuilderPackageTarget(PackageTarget::from_str("x86_64-linux").unwrap());
    }

    lazy_static! {
        static ref TARGET_LINUX: BuilderPackageTarget =
            BuilderPackageTarget(PackageTarget::from_str("x86_64-linux").unwrap());
        static ref TARGET_WINDOWS: BuilderPackageTarget =
            BuilderPackageTarget(PackageTarget::from_str("x86_64-windows").unwrap());
    }

    fn make_job_graph_entry(id: i64) -> JobGraphEntry {
        JobGraphEntry { id,
                        group_id: 0,
                        job_state: JobExecState::Pending,
                        plan_ident: "dummy_plan_ident".to_owned(),
                        manifest_ident: "dummy_manifest_ident".to_owned(),
                        as_built_ident: None,
                        dependencies: vec![],
                        waiting_on_count: 0,
                        target_platform:
                            BuilderPackageTarget(PackageTarget::from_str("x86_64-linux").unwrap()),
                        created_at: Utc.timestamp(1431648000, 0),
                        updated_at: Utc.timestamp(1431648001, 0) }
    }

    #[tokio::test]
    async fn simple_job_fetch() {
        let datastores = setup_simple_job_fetch();

        for store in datastores {
            let (mut s_tx, s_rx) = tokio::sync::mpsc::channel(1);
            let (wrk_tx, _wrk_rx) = tokio::sync::mpsc::channel(1);
            let mut scheduler = Scheduler::new(store, s_rx, wrk_tx);
            let join = tokio::task::spawn(async move { scheduler.run().await });

            // expect a job for this target
            let (o_tx, o_rx) = oneshot::channel::<Option<JobGraphId>>();
            let _ =
                s_tx.send(SchedulerMessage::WorkerNeedsWork { worker:
                                                                  WorkerId("worker1".to_string()),
                                                              target: *TARGET_LINUX,
                                                              reply:  o_tx, })
                    .await;

            let reply: Option<JobGraphId> = o_rx.await.unwrap();
            println!("Reply 1 {:?}", reply);
            assert_eq!(1, reply.unwrap().0);

            // No job for this target
            let (o_tx, o_rx) = oneshot::channel::<Option<JobGraphId>>();
            let _ =
                s_tx.send(SchedulerMessage::WorkerNeedsWork { worker:
                                                                  WorkerId("worker1".to_string()),
                                                              target: *TARGET_WINDOWS,
                                                              reply:  o_tx, })
                    .await;

            let reply: Option<JobGraphId> = o_rx.await.unwrap();
            println!("Reply 2 {:?}", reply);
            assert_eq!(None, reply);

            drop(s_tx);
            join.await.unwrap();
        }
    }

    fn setup_simple_job_fetch() -> Vec<Box<dyn SchedulerDataStore>> {
        let mut stores: Vec<Box<dyn SchedulerDataStore>> = Vec::new();
        {
            let actions =
                vec![(DummySchedulerDataStoreCall::TakeNextJobForTarget { target: *TARGET_LINUX, },
                      DummySchedulerDataStoreResult::JobOption(Ok(Some(make_job_graph_entry(0))))),
                     (DummySchedulerDataStoreCall::TakeNextJobForTarget { target: *TARGET_WINDOWS, },
                      DummySchedulerDataStoreResult::JobOption(Ok(None)))];

            let _dummy_store = Box::new(DummySchedulerDataStore::new(actions));
            // stores.push(dummy_store);
        }

        #[cfg(feature = "postgres_scheduler_tests")]
        {
            let database = SchedulerDataStoreDb::new_test();
            let conn = database.get_connection_for_test();
            let entry = NewJobGraphEntry { group_id:         0,
                                           job_state:        JobExecState::Ready,
                                           plan_ident:       "dummy_plan_ident",
                                           manifest_ident:   "dummy_manifest_ident",
                                           as_built_ident:   None,
                                           dependencies:     &[],
                                           waiting_on_count: 0,
                                           target_platform:  &TARGET_LINUX, };
            let e = JobGraphEntry::create(&entry, &conn).unwrap();
            assert_eq!(1, e.id);
            stores.push(Box::new(database));
        }

        stores
    }
}
