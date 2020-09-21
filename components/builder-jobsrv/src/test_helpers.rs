use crate::hab_core::package::PackageTarget;

use habitat_builder_db::models::{jobs::{JobExecState,
                                        JobGraphEntry,
                                        NewJobGraphEntry},
                                 package::BuilderPackageTarget};

use chrono::{DateTime,
             Duration,
             TimeZone,
             Utc};

use std::{collections::HashMap,
          str::FromStr};

use lazy_static::lazy_static;

lazy_static! {
    pub static ref TARGET_PLATFORM: BuilderPackageTarget =
        BuilderPackageTarget(PackageTarget::from_str("x86_64-linux").unwrap());
    pub static ref TARGET_LINUX: BuilderPackageTarget =
        BuilderPackageTarget(PackageTarget::from_str("x86_64-linux").unwrap());
    pub static ref TARGET_WINDOWS: BuilderPackageTarget =
        BuilderPackageTarget(PackageTarget::from_str("x86_64-windows").unwrap());
}

#[macro_export]
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

#[allow(dead_code)]
pub fn is_recent(time: Option<DateTime<Utc>>, tolerance: isize) -> bool {
    Utc::now() - time.unwrap() < Duration::seconds(tolerance as i64)
}

// We expect things to have the same time, but sometimes rounding bites us
#[allow(dead_code)]
pub fn about_same_time(left: Option<DateTime<Utc>>, right: DateTime<Utc>) -> bool {
    (left.unwrap().timestamp_millis() - right.timestamp_millis()).abs() < 100
}

#[derive(Default, Debug, Clone)]
pub struct JobStateCounts {
    pub p:  i64,
    pub wd: i64,
    pub rd: i64,
    pub rn: i64,
    pub c:  i64,
    pub jf: i64,
    pub df: i64,
    pub cp: i64,
    pub cc: i64,
}
// TODO REPLACE WITH Version in jobs
pub fn job_state_count_s(gid: i64, conn: &diesel::pg::PgConnection) -> JobStateCounts {
    let mut j = JobStateCounts::default();
    j.p = JobGraphEntry::count_by_state(gid, JobExecState::Pending, &conn).unwrap();
    j.wd = JobGraphEntry::count_by_state(gid, JobExecState::WaitingOnDependency, &conn).unwrap();
    j.rd = JobGraphEntry::count_by_state(gid, JobExecState::Ready, &conn).unwrap();
    j.rn = JobGraphEntry::count_by_state(gid, JobExecState::Running, &conn).unwrap();
    j.c = JobGraphEntry::count_by_state(gid, JobExecState::Complete, &conn).unwrap();
    j.jf = JobGraphEntry::count_by_state(gid, JobExecState::JobFailed, &conn).unwrap();
    j.df = JobGraphEntry::count_by_state(gid, JobExecState::DependencyFailed, &conn).unwrap();
    j
}

pub fn job_state_count(gid: i64, conn: &diesel::pg::PgConnection) -> (i64, i64, i64, i64, i64) {
    let ready = JobGraphEntry::count_by_state(gid, JobExecState::Ready, &conn).unwrap();
    let waiting_on_dependency =
        JobGraphEntry::count_by_state(gid, JobExecState::WaitingOnDependency, &conn).unwrap();
    let complete = JobGraphEntry::count_by_state(gid, JobExecState::Complete, &conn).unwrap();
    let failed = JobGraphEntry::count_by_state(gid, JobExecState::JobFailed, &conn).unwrap();
    let dep_failed =
        JobGraphEntry::count_by_state(gid, JobExecState::DependencyFailed, &conn).unwrap();

    (waiting_on_dependency, ready, complete, failed, dep_failed)
}

pub fn make_job_graph_entry(id: i64) -> JobGraphEntry {
    JobGraphEntry { id,
                    group_id: 0,
                    job_state: JobExecState::Pending,
                    project_id: 0,
                    job_id: None,
                    manifest_ident: "dummy_manifest_ident".to_owned(),
                    as_built_ident: None,
                    dependencies: vec![],
                    waiting_on_count: 0,
                    target_platform:
                        BuilderPackageTarget(PackageTarget::from_str("x86_64-linux").unwrap()),
                    created_at: Utc.timestamp(1431648000, 0),
                    updated_at: Utc.timestamp(1431648001, 0) }
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
                                       project_id: 0,
                                       job_id: None,
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

    #[allow(dead_code)]
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

    h.add(conn, "foo/bar/1.2.3/123", &[], JobExecState::Ready);
    h.add(conn,
          "foo/baz/1.2.3/123",
          &["foo/bar/1.2.3/123"],
          JobExecState::WaitingOnDependency);
    h.add(conn,
          "foo/ping/1.2.3/123",
          &["foo/bar/1.2.3/123"],
          JobExecState::WaitingOnDependency);
    h.add(conn,
          "foo/pong/1.2.3/123",
          &["foo/baz/1.2.3/123", "foo/ping/1.2.3/123"],
          JobExecState::WaitingOnDependency);

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
                                       project_id: 0, /* TODO maybe lookup project based on
                                                       * plan ident, and substitute right
                                                       * value */
                                       job_id: None,
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
