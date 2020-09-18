#[cfg(test)]
#[cfg(feature = "postgres_scheduler_tests")]
// cargo test --features postgres_scheduler_tests to enable
// from root
// cargo test -p habitat_builder_jobsrv --features=postgres_scheduler_tests
// --manifest-path=components/builder-jobsrv/Cargo.toml
mod test {

    use super::super::*;

    use crate::{assert_match,
                db::models::{jobs::{Group,
                                    JobExecState,
                                    JobGraphEntry,
                                    JobStateCounts,
                                    NewGroup,
                                    NewJobGraphEntry},
                             package::BuilderPackageTarget},
                scheduler_datastore::{DummySchedulerDataStore,
                                      DummySchedulerDataStoreCall,
                                      DummySchedulerDataStoreResult,
                                      JobGraphId,
                                      SchedulerDataStore,
                                      SchedulerDataStoreDb,
                                      WorkerId},
                test_helpers::*};

    fn setup_scheduler(data_store: Box<dyn SchedulerDataStore>) -> (Scheduler, JoinHandle<()>) {
        let (s_tx, s_rx) = tokio::sync::mpsc::channel(1);
        let (wrk_tx, _wrk_rx) = tokio::sync::mpsc::channel(1);

        let scheduler = Scheduler::new(s_tx);
        let join = Scheduler::start(data_store, s_rx, wrk_tx);
        (scheduler, join)
    }

    #[tokio::test]
    async fn test_state() {
        let store = Box::new(DummySchedulerDataStore::new(Vec::new()));
        let (mut scheduler, join) = setup_scheduler(store);

        println!("Want the state 1");
        let reply1 = scheduler.state().await;
        println!("Reply 1 {:?}", reply1);
        assert_eq!(1, reply1.message_count);
        assert_eq!("", reply1.last_message_debug);

        println!("Want the state 2");
        let reply2 = scheduler.state().await;
        println!("Reply 2 {:?}", reply2);
        assert_eq!(2, reply2.message_count);

        // We expect the scheduler loop to render exactly what we sent it, but we can't
        // see that because sending mutates it. (the oneshot  is_rx_task_set changes state)
        assert_ne!("", reply2.last_message_debug);

        drop(scheduler);
        join.await.unwrap();
    }

    #[tokio::test]
    async fn simple_job_group_added() {
        let datastore = setup_simple_job_group_added();
        let conn = &datastore.get_connection_for_test();
        let store = Box::new(datastore);

        let (mut scheduler, join) = setup_scheduler(store);

        // for reasons, we can deterministically generate JobGraphEntry ids but not group ids, so we
        // fetch it
        let entry = JobGraphEntry::get(1, &conn).unwrap();
        let gid = GroupId(entry.group_id);

        let states = JobGraphEntry::count_all_states(gid.0, &conn).unwrap();
        assert_match!(states, crate::db::models::jobs::JobStateCounts{ pd : 2, wd : 0, rd :0, rn : 0, ..});

        scheduler.job_group_added(gid, *TARGET_LINUX).await;
        scheduler.state().await; // make sure scheduler has finished work

        let states = JobGraphEntry::count_all_states(gid.0, &conn).unwrap();
        assert_match!(states, crate::db::models::jobs::JobStateCounts{ pd : 0, wd : 1, rd : 1, rn : 0, ..});

        drop(scheduler);
        join.await.unwrap();
    }

    fn setup_simple_job_group_added() -> SchedulerDataStoreDb {
        let database = SchedulerDataStoreDb::new_test();
        let conn = database.get_connection_for_test();

        let target = TARGET_LINUX.0.to_string();

        let new_group = NewGroup { group_state:  "Queued",
                                   project_name: "monkeypants",
                                   target:       &target, };
        let group = Group::create(&new_group, &conn).unwrap();

        let entry = NewJobGraphEntry { group_id:         group.id,
                                       job_state:        JobExecState::Pending,
                                       plan_ident:       "dummy_plan_ident",
                                       manifest_ident:   "dummy_manifest_ident",
                                       as_built_ident:   None,
                                       dependencies:     &[],
                                       waiting_on_count: 0,
                                       target_platform:  &TARGET_LINUX, };
        let e1 = JobGraphEntry::create(&entry, &conn).unwrap();
        assert_eq!(1, e1.id);

        let entry = NewJobGraphEntry { group_id:         group.id,
                                       job_state:        JobExecState::Pending,
                                       plan_ident:       "dummy_plan_ident2",
                                       manifest_ident:   "dummy_manifest_ident2",
                                       as_built_ident:   None,
                                       dependencies:     &[e1.id],
                                       waiting_on_count: 1,
                                       target_platform:  &TARGET_LINUX, };
        let e2 = JobGraphEntry::create(&entry, &conn).unwrap();
        assert_eq!(2, e2.id);

        database
    }

    #[tokio::test]
    async fn simple_job_failed() {
        let datastore = setup_simple_job_complete();
        let conn = &datastore.get_connection_for_test();
        let store = Box::new(datastore);
        let worker = WorkerId("test-worker".to_string());

        let (mut scheduler, join) = setup_scheduler(store);

        // for reasons, we can deterministically generate JobGraphEntry ids but not group ids, so we
        // fetch it
        let entry = JobGraphEntry::get(1, &conn).unwrap();
        let gid = GroupId(entry.group_id);

        let states = JobGraphEntry::count_all_states(gid.0, &conn).unwrap();
        assert_match!(states, crate::db::models::jobs::JobStateCounts{ pd : 0, wd : 1, rd :0, rn : 1, ..});

        scheduler.worker_finished(worker.clone(), JobGraphId(1), JobExecState::JobFailed)
                 .await;
        scheduler.state().await; // make sure scheduler has finished work

        let states = JobGraphEntry::count_all_states(gid.0, &conn).unwrap();
        assert_match!(states, crate::db::models::jobs::JobStateCounts{ pd : 0, wd : 0, rd : 0, rn : 0, ct: 0, jf: 1, df: 1, ..});

        // TODO: We don't handle group transition yet. Maybe this should be a seperate test?
        // let states = JobGraphEntry::count_all_states(gid.0, &conn).unwrap();
        // assert_match!(states, crate::db::models::jobs::JobStateCounts{ pd : 0, wd : 0, rd : 0, rn
        // : 0, ct: 2, ..});

        // let group = Group::get(gid.0, &conn).unwrap();
        // assert_eq!(group.group_state, "Complete");

        drop(scheduler);
        join.await.unwrap();
    }

    #[tokio::test]
    async fn simple_job_complete() {
        let datastore = setup_simple_job_complete();
        let conn = &datastore.get_connection_for_test();
        let store = Box::new(datastore);
        let worker = WorkerId("test-worker".to_string());

        let (mut scheduler, join) = setup_scheduler(store);

        // for reasons, we can deterministically generate JobGraphEntry ids but not group ids, so we
        // fetch it
        let entry = JobGraphEntry::get(1, &conn).unwrap();
        let gid = GroupId(entry.group_id);

        let states = JobGraphEntry::count_all_states(gid.0, &conn).unwrap();
        assert_match!(states, crate::db::models::jobs::JobStateCounts{ pd : 0, wd : 1, rd :0, rn : 1, ..});

        scheduler.worker_finished(worker.clone(), JobGraphId(1), JobExecState::Complete)
                 .await;
        scheduler.state().await; // make sure scheduler has finished work

        let states = JobGraphEntry::count_all_states(gid.0, &conn).unwrap();
        assert_match!(states, crate::db::models::jobs::JobStateCounts{ pd : 0, wd : 0, rd : 1, rn : 0, ct: 1, ..});

        // TODO: We don't handle group transition yet. Maybe this should be a seperate test?
        // let states = JobGraphEntry::count_all_states(gid.0, &conn).unwrap();
        // assert_match!(states, crate::db::models::jobs::JobStateCounts{ pd : 0, wd : 0, rd : 0, rn
        // : 0, ct: 2, ..});

        // let group = Group::get(gid.0, &conn).unwrap();
        // assert_eq!(group.group_state, "Complete");

        drop(scheduler);
        join.await.unwrap();
    }

    fn setup_simple_job_complete() -> SchedulerDataStoreDb {
        let database = SchedulerDataStoreDb::new_test();
        let conn = database.get_connection_for_test();

        let target = TARGET_LINUX.0.to_string();

        let new_group = NewGroup { group_state:  "Dispatching",
                                   project_name: "monkeypants",
                                   target:       &target, };
        let group = Group::create(&new_group, &conn).unwrap();

        let entry = NewJobGraphEntry { group_id:         group.id,
                                       job_state:        JobExecState::Running,
                                       plan_ident:       "dummy_plan_ident",
                                       manifest_ident:   "dummy_manifest_ident",
                                       as_built_ident:   None,
                                       dependencies:     &[],
                                       waiting_on_count: 0,
                                       target_platform:  &TARGET_LINUX, };
        let e1 = JobGraphEntry::create(&entry, &conn).unwrap();
        assert_eq!(1, e1.id);

        let entry = NewJobGraphEntry { group_id:         group.id,
                                       job_state:        JobExecState::WaitingOnDependency,
                                       plan_ident:       "dummy_plan_ident2",
                                       manifest_ident:   "dummy_manifest_ident2",
                                       as_built_ident:   None,
                                       dependencies:     &[e1.id],
                                       waiting_on_count: 1,
                                       target_platform:  &TARGET_LINUX, };
        let e2 = JobGraphEntry::create(&entry, &conn).unwrap();
        assert_eq!(2, e2.id);

        database
    }

    #[tokio::test]
    async fn simple_job_fetch() {
        let datastores = setup_simple_job_fetch();

        for store in datastores {
            let (mut s_tx, s_rx) = tokio::sync::mpsc::channel(1);
            let (wrk_tx, _wrk_rx) = tokio::sync::mpsc::channel(1);
            let mut scheduler = SchedulerInternal::new(store, s_rx, wrk_tx);
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

    #[tokio::test]
    async fn dispatching_job_fetch() {
        let datastores = setup_dispatching_job_fetch();

        for store in datastores {
            let (mut s_tx, s_rx) = tokio::sync::mpsc::channel(1);
            let (wrk_tx, _wrk_rx) = tokio::sync::mpsc::channel(1);
            let mut scheduler = SchedulerInternal::new(store, s_rx, wrk_tx);
            let join = tokio::task::spawn(async move { scheduler.run().await });

            // Nothing ready, but a group available

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

            // expect the group to be moved to Dispatching here
            // TODO write api to let us test that...

            // No job and no group available for this target
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

    fn setup_dispatching_job_fetch() -> Vec<Box<dyn SchedulerDataStore>> {
        let mut stores: Vec<Box<dyn SchedulerDataStore>> = Vec::new();

        #[cfg(feature = "to_be_implemented")] // TODO write the Dummy impl.
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

            let target = TARGET_LINUX.0.to_string();

            let new_group = NewGroup { group_state:  "Queued",
                                       project_name: "monkeypants",
                                       target:       &target, };
            let group = Group::create(&new_group, &conn).unwrap();

            let entry = NewJobGraphEntry { group_id:         group.id,
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
