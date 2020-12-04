use super::*;

use crate::{assert_match,
            test_helpers::*};

use habitat_builder_db::{datastore_test,
                         models::jobs::{JobExecState,
                                        NewJobGraphEntry}};
use std::str::FromStr;

mod helpers {
    #[allow(dead_code)]
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
}

#[test]
fn create_job_graph_entry() {
    let target_platform = *TARGET_LINUX;
    let ds = datastore_test!(DataStore);
    let conn = ds.get_pool().get_conn().unwrap();
    let entry = NewJobGraphEntry::new(0,
                                      "foo/bar",
                                      "foo/bar/1.2.3/123",
                                      JobExecState::Pending,
                                      &[1, 2, 3],
                                      target_platform);

    let job_graph_entry = JobGraphEntry::create(&entry, &conn).unwrap();

    assert_eq!(job_graph_entry.group_id, 0);
    assert_eq!(job_graph_entry.job_state, JobExecState::Pending);
}

#[test]
fn take_next_job_for_target() {
    let target_platform = *TARGET_LINUX;

    let ds = datastore_test!(DataStore);
    let conn = ds.get_pool().get_conn().unwrap();

    let mut h = DbHelper::new(0, &TARGET_LINUX);
    h.add(&conn, "foo/bar/1.2.3/123", &[], JobExecState::Pending);
    h.add(&conn,
          "foo/baz/1.2.3/123",
          &[],
          JobExecState::WaitingOnDependency);
    h.add(&conn, "foo/ping/1.2.3/123", &[], JobExecState::Ready);

    let mut h_alt = DbHelper::new(1, &TARGET_WINDOWS);
    h_alt.add(&conn,
              "foo/pong/1.2.3/123",
              &[],
              JobExecState::WaitingOnDependency);

    let job_next = JobGraphEntry::take_next_job_for_target(target_platform, &conn).unwrap();
    assert!(job_next.is_some());
    assert_eq!(job_next.unwrap().id, h.id_by_name("foo/ping/1.2.3/123"));
    // TODO verify we update the state to 'dispatched'

    let job_next = JobGraphEntry::take_next_job_for_target(target_platform, &conn).unwrap();
    assert!(job_next.is_none());
}

// This is the start of a test, but isn't complete. However it is useful for some
// manual debugging tasks, so
// #[test]
#[allow(dead_code)]
fn insert_many_jobs() {
    let target_platform = *TARGET_LINUX;
    let ds = datastore_test!(DataStore);
    let conn = ds.get_pool().get_conn().unwrap();

    let manifest = helpers::manifest_data_from_file();
    for i in 1..2 {
        make_job_graph_entries(i as i64,
                               JobExecState::Ready,
                               target_platform,
                               &manifest,
                               &conn);
    }

    assert_eq!(JobGraphEntry::count_by_state(0, JobExecState::Ready, &conn).unwrap(),
               0);
    // std::thread::sleep(std::time::Duration::from_secs(10000));
}

#[test]
fn count_by_state() {
    let ds = datastore_test!(DataStore);
    let conn = ds.get_pool().get_conn().unwrap();

    let group_id = 1;
    let _ = make_simple_graph_helper(0, &TARGET_PLATFORM, &conn);

    assert_eq!(JobGraphEntry::count_by_state(group_id, JobExecState::Ready, &conn).unwrap(),
               0);
    assert_eq!(JobGraphEntry::count_by_state(group_id,
                                                 JobExecState::WaitingOnDependency,
                                                 &conn).unwrap(),
                   0);
    assert_eq!(JobGraphEntry::count_by_state(group_id, JobExecState::Complete, &conn).unwrap(),
               0);

    make_simple_graph_helper(group_id, &TARGET_PLATFORM, &conn);

    assert_eq!(JobGraphEntry::count_by_state(group_id,
                                                 JobExecState::WaitingOnDependency,
                                                 &conn).unwrap(),
                   3);
    assert_eq!(JobGraphEntry::count_by_state(group_id, JobExecState::Ready, &conn).unwrap(),
               1);
    assert_eq!(JobGraphEntry::count_by_state(group_id, JobExecState::Complete, &conn).unwrap(),
               0);
}

#[test]
fn transitive_rdeps_for_id_diamond() {
    let ds = datastore_test!(DataStore);
    let conn = ds.get_pool().get_conn().unwrap();

    let _ = make_simple_graph_helper(0, &TARGET_PLATFORM, &conn);

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

    let _ = make_simple_graph_helper(0, &TARGET_PLATFORM, &conn);

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
fn mark_job_failed() {
    let ds = datastore_test!(DataStore);
    let conn = ds.get_pool().get_conn().unwrap();

    make_simple_graph_helper(0, &TARGET_PLATFORM, &conn);

    // id starts at 1
    let count = JobGraphEntry::mark_job_failed(1, &conn).unwrap();
    // TODO: Should this reflect _all_ things marked failed or
    // only failed dependencies?
    assert_eq!(count, 3);

    assert_match!(JobGraphEntry::count_all_states(0, &conn).unwrap(),
                  JobStateCounts { pd: 0,
                                   wd: 0,
                                   rd: 0,
                                   rn: 0,
                                   ct: 0,
                                   jf: 1,
                                   df: 3,
                                   cp: 0,
                                   cc: 0, });
}

#[test]
// TODO: Is it worth setting up the states to reflect jobs that would
// need to be completed and the target for failure would be running?
fn mark_job_failed_partial_group() {
    let ds = datastore_test!(DataStore);
    let conn = ds.get_pool().get_conn().unwrap();

    make_simple_graph_helper(0, &TARGET_PLATFORM, &conn);

    let count = JobGraphEntry::mark_job_failed(2, &conn).unwrap();
    println!("{}", count);
    // TODO: Should this reflect _all_ things marked failed or
    // only failed dependencies?
    assert_eq!(count, 1);

    assert_match!(JobGraphEntry::count_all_states(0, &conn).unwrap(),
                  JobStateCounts { pd: 0,
                                   wd: 1, // Opposite side of the failed
                                   rd: 1, // Root of the diamond
                                   rn: 0,
                                   ct: 0,
                                   jf: 1,
                                   df: 1,
                                   cp: 0,
                                   cc: 0, });
}

#[test]
fn mark_job_complete() {
    let ds = datastore_test!(DataStore);
    let conn = ds.get_pool().get_conn().unwrap();

    make_simple_graph_helper(1, &TARGET_PLATFORM, &conn);
    make_simple_graph_helper(2, &TARGET_PLATFORM, &conn); // This group should not be scheduled

    // We prefer group 1 while there is work left; then group 2
    let job_next = JobGraphEntry::take_next_job_for_target(*TARGET_PLATFORM, &conn).unwrap();
    assert!(job_next.is_some());
    let job_data = job_next.unwrap();
    assert_eq!(job_data.manifest_ident, "foo/bar/1.2.3/123");
    assert_eq!(job_data.group_id, 1);

    let as_built = BuilderPackageIdent::from_str("foo/bar/1.2.3/123").unwrap();
    let ready = JobGraphEntry::mark_job_complete(job_data.id, &as_built, &conn);
    assert_eq!(ready.unwrap(), 2);

    assert_eq!((1, 2, 1, 0, 0), job_state_count(1, &conn));

    let job_next = JobGraphEntry::take_next_job_for_target(*TARGET_PLATFORM, &conn).unwrap();
    assert!(job_next.is_some());
    let job_data = job_next.unwrap();
    assert_eq!(job_data.group_id, 1);

    let as_built = BuilderPackageIdent::from_str("foo/dummydata/1.2.3/123").unwrap(); // not really correct
    let ready = JobGraphEntry::mark_job_complete(job_data.id, &as_built, &conn);
    assert_eq!(ready.unwrap(), 0);

    assert_eq!((1, 1, 2, 0, 0), job_state_count(1, &conn));

    let job_next = JobGraphEntry::take_next_job_for_target(*TARGET_PLATFORM, &conn).unwrap();
    assert!(job_next.is_some());
    let job_data = job_next.unwrap();
    assert_eq!(job_data.group_id, 1);

    let as_built = BuilderPackageIdent::from_str("foo/dummydata/1.2.3/123").unwrap(); // not really correct
    let ready = JobGraphEntry::mark_job_complete(job_data.id, &as_built, &conn);
    assert_eq!(ready.unwrap(), 1);

    assert_eq!((0, 1, 3, 0, 0), job_state_count(1, &conn));

    let job_next = JobGraphEntry::take_next_job_for_target(*TARGET_PLATFORM, &conn).unwrap();
    assert!(job_next.is_some());
    let job_data = job_next.unwrap();
    assert_eq!(job_data.group_id, 1);

    let as_built = BuilderPackageIdent::from_str("foo/dummydata/1.2.3/123").unwrap(); // not really correct
    let ready = JobGraphEntry::mark_job_complete(job_data.id, &as_built, &conn);
    assert_eq!(ready.unwrap(), 0);

    assert_eq!((0, 0, 4, 0, 0), job_state_count(1, &conn));

    let job_next = JobGraphEntry::take_next_job_for_target(*TARGET_PLATFORM, &conn).unwrap();
    assert!(job_next.is_some());
    let job_data = job_next.unwrap();
    assert_eq!(job_data.group_id, 2);
}

#[test]
fn mark_job_complete_interleaved() {
    let ds = datastore_test!(DataStore);
    let conn = ds.get_pool().get_conn().unwrap();

    make_simple_graph_helper(1, &TARGET_PLATFORM, &conn);
    make_simple_graph_helper(2, &TARGET_PLATFORM, &conn); // This group should not be scheduled

    let job_next = JobGraphEntry::take_next_job_for_target(*TARGET_PLATFORM, &conn).unwrap();
    assert!(job_next.is_some());
    let job_data = job_next.unwrap();
    assert_eq!(job_data.manifest_ident, "foo/bar/1.2.3/123");
    assert_eq!(job_data.group_id, 1);

    let as_built = BuilderPackageIdent::from_str("foo/dummydata/1.2.3/123").unwrap(); // not really correct
    let ready = JobGraphEntry::mark_job_complete(job_data.id, &as_built, &conn);
    assert_eq!(ready.unwrap(), 2);

    assert_match!(JobGraphEntry::count_all_states(1, &conn).unwrap(),
                      JobStateCounts { pd: 0, wd: 1, rd: 2, rn: 0, ct: 1, .. });
    assert_match!(JobGraphEntry::count_all_states(2, &conn).unwrap(),
                      JobStateCounts {pd: 0, wd: 3, rd: 1, .. });

    // Get another job from group 1
    let job_a = JobGraphEntry::take_next_job_for_target(*TARGET_PLATFORM, &conn).unwrap()
                                                                                .unwrap();
    assert_eq!(job_a.group_id, 1);
    assert_match!(JobGraphEntry::count_all_states(1, &conn).unwrap(),
                      JobStateCounts { pd: 0, wd: 1, rd: 1, rn: 1, ct: 1, .. });
    assert_match!(JobGraphEntry::count_all_states(2, &conn).unwrap(),
                      JobStateCounts {pd: 0, wd: 3, rd: 1, .. });

    // Get another job, expect group 1
    let job_b = JobGraphEntry::take_next_job_for_target(*TARGET_PLATFORM, &conn).unwrap()
                                                                                .unwrap();
    assert_eq!(job_b.group_id, 1);
    assert_eq!((1, 0, 1, 0, 0), job_state_count(1, &conn));
    assert_eq!((3, 1, 0, 0, 0), job_state_count(2, &conn));

    // There are no more group one jobs, so expect group 2
    let job_c = JobGraphEntry::take_next_job_for_target(*TARGET_PLATFORM, &conn).unwrap()
                                                                                .unwrap();

    assert_eq!(job_c.group_id, 2);
    assert_eq!((1, 0, 1, 0, 0), job_state_count(1, &conn));
    assert_eq!((3, 0, 0, 0, 0), job_state_count(2, &conn));

    let as_built = BuilderPackageIdent::from_str("foo/dummydata/1.2.3/123").unwrap(); // not really correct
    let ready = JobGraphEntry::mark_job_complete(job_a.id, &as_built, &conn);
    assert_eq!(ready.unwrap(), 0);
    assert_eq!((1, 0, 2, 0, 0), job_state_count(1, &conn));
    assert_eq!((3, 0, 0, 0, 0), job_state_count(2, &conn));

    let as_built = BuilderPackageIdent::from_str("foo/dummydata/1.2.3/123").unwrap(); // not really correct
    let ready = JobGraphEntry::mark_job_complete(job_b.id, &as_built, &conn);
    assert_eq!(ready.unwrap(), 1);
    assert_eq!((0, 1, 3, 0, 0), job_state_count(1, &conn));
    assert_eq!((3, 0, 0, 0, 0), job_state_count(2, &conn));

    let as_built = BuilderPackageIdent::from_str("foo/dummydata/1.2.3/123").unwrap(); // not really correct
    let ready = JobGraphEntry::mark_job_complete(job_c.id, &as_built, &conn);
    assert_eq!(ready.unwrap(), 2);
    assert_eq!((0, 1, 3, 0, 0), job_state_count(1, &conn));
    assert_eq!((1, 2, 1, 0, 0), job_state_count(2, &conn));

    let job_next = JobGraphEntry::take_next_job_for_target(*TARGET_PLATFORM, &conn).unwrap()
                                                                                   .unwrap();
    assert_eq!(job_next.group_id, 1);

    let as_built = BuilderPackageIdent::from_str("foo/dummydata/1.2.3/123").unwrap(); // not really correct
    let ready = JobGraphEntry::mark_job_complete(job_next.id, &as_built, &conn);
    assert_eq!(ready.unwrap(), 0);
    assert_eq!((0, 0, 4, 0, 0), job_state_count(1, &conn));
    assert_eq!((1, 2, 1, 0, 0), job_state_count(2, &conn));
}
