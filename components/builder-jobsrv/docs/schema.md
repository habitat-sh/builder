= Vocabulary

A change (PR, package upload) affects one or more plans

Each plan touched (either directly or by hinting in bldr.toml) creates
a group per target. 

= Open questions

Microservice philosophy; Should the jobsrv tables be totally isolated (they aren't today)
Should we move jobs.rs diesel over to jobsrv?


Where do we keep logs from builds?


= Tables

== groups

There is a group for every pair of touched entity and target. So if an
update modifies multiple projects, there would be a separate group for
each project.

    Column    |           Type           |                       Modifiers                        
--------------+--------------------------+--------------------------------------------------------
 id           | bigint                   | not null default next_id_v1('groups_id_seq'::regclass) <- other tables use this to point 
 group_state  | text                     | <- enum JobGroupState in jobsrv.rs
 project_name | text                     | 
 created_at   | timestamp with time zone | default now()
 updated_at   | timestamp with time zone | default now()
 target       | text                     | default 'x86_64-linux'::text
Indexes:
    "groups_pkey" PRIMARY KEY, btree (id)
    "pending_groups_index_v1" btree (created_at) WHERE group_state = 'Pending'::text
    "queued_groups_index_v1" btree (created_at) WHERE group_state = 'Queued'::text

Investigate single index on group_state, created_at

=== id
We leak this field as a group id in the API

=== group_state
Represents the lifecycle of a build
Matches to enum JobGroupState in jobsrv.rs

pub enum JobGroupState {
    GroupPending = 0,
    GroupDispatching = 1,
    GroupComplete = 2,
    GroupFailed = 3,
    GroupQueued = 4,
    GroupCanceled = 5,
}


Interesting item found when inspecting the actual values: running
```SELECT DISTINCT group_state FROM groups;``` found no Failed jobs.

It appears that we may be representing Failed jobs as Complete; this
may be a bug as we promote complete jobs. Possibly intentional. Have
to inspect individual job state to decide if group was promotable.



=== project_name
Format: ORIGIN/PACKAGE_NAME, may eventually come from schema origin_projects.name
This could have been a fk to origin_projects.id; stems from historical separation of jobsrv

=== created_at
We rely on default now for this

=== updated_at
Some stored procedures update this, but we should move diesel triggers for update.

== group_projects
Why does this exist? The only unique information is project_state AFAIK. Current best guess that
is created when we first materialized the group, and the job is only created when things are distpatched.
TODO research this more.


    Column     |           Type           |                          Modifiers                          
---------------+--------------------------+-------------------------------------------------------------
 id            | bigint                   | not null default nextval('group_projects_id_seq'::regclass)
 owner_id      | bigint                   | <- groups.id
 project_name  | text                     | 
 project_ident | text                     | 
 project_state | text                     | <- enum JobGroupProjectState in jobsrv.rs
 job_id        | bigint                   | default 0
 created_at    | timestamp with time zone | default now()
 updated_at    | timestamp with time zone | default now()
 target        | text                     | default 'x86_64-linux'::text
Indexes:
    "group_projects_pkey" PRIMARY KEY, btree (id)


=== owner_id
Should be FK to groups.id

=== project_name
Format: ORIGIN/PACKAGE_NAME, may eventually come from schema origin_projects.name
This could have been a fk to origin_projects.id; stems from historical separation of jobsrv

=== project_ident
FQPI for built package, or the prior build of that package; e.g. if a 

=== project_state
enum JobGroupProjectState in jobsrv.rs

pub enum JobGroupProjectState {
    NotStarted = 0,
    InProgress = 1,
    Success = 2,
    Failure = 3,
    Skipped = 4,
    Canceled = 5,
}

=== job_id
jobs.id for the package,

== jobs

This represents a single package build (and container build
probably). Many of these comprise a group.

Question: How is container build represented? Tacked onto the end of
the package build (how is represented?) What is the right way to built
containers? Is it something we want to separate out? Do we build a
container every time it's underlying package is built? Or is it on
promotion.


This copies a lot of stuff from origin_projects, but names the fields
slightly differently. Not clear how this data winds up here; is it
from the protobuf message or something else?

                                        Table "public.jobs"
      Column       |           Type           |                      Modifiers                      
-------------------+--------------------------+-----------------------------------------------------
 id                | bigint                   | not null default next_id_v1('job_id_seq'::regclass)
 owner_id          | bigint                   | <- groups.id
 job_state         | text                     | default 'Pending'::text <- enum JobState in jobsrv.rs
 project_id        | bigint                   | 
 project_name      | text                     | 
 project_owner_id  | bigint                   | 
 project_plan_path | text                     |
 vcs               | text                     | 
 vcs_arguments     | text[]                   | 
 net_error_code    | integer                  | 
 net_error_msg     | text                     | 
 scheduler_sync    | boolean                  | default false
 created_at        | timestamp with time zone | default now()
 updated_at        | timestamp with time zone | default now()
 build_started_at  | timestamp with time zone | 
 build_finished_at | timestamp with time zone | 
 package_ident     | text                     | 
 archived          | boolean                  | not null default false
 channel           | text                     | 
 sync_count        | integer                  | default 0
 worker            | text                     | 
 target            | text                     | default 'x86_64-linux'::text
Indexes:
    "jobs_pkey" PRIMARY KEY, btree (id)
    "pending_jobs_index_v1" btree (created_at) WHERE job_state = 'Pending'::text

=== id
No FK relationships

=== owner_id
This is the groups.id, and should have a strict FK relationship with it.

=== job_state
enum JobState in jobsrv.rs

pub enum JobState {
    Pending = 0,
    Processing = 1,
    Complete = 2,
    Rejected = 3,
    Failed = 4,
    Running = 5,
    CancelPending = 6,
    CancelProcessing = 7,
    CancelComplete = 8,
}

=== project_id
origin_project.id (where does this come from? Are we reading origin projects in jobsrv?

=== project_name TODO 
Format: ORIGIN/PACKAGE_NAME, may eventually come from schema origin_projects.name
This could have been a fk to origin_projects.id; stems from historical separation of jobsrv

=== project_owner_id
taken from origin_projects.owner_id

Question: How is this captured and managed in origin_projects? Is this part of a protobuf message, or are we mining the tables. Mostly likely this is from reading the origin_projects table in jobsrv, because we can't know what projects we're rebuilding without the graph.

=== project_plan_path
Maps to origin_projects.plan_path

=== vcs
VCS type, always git.

=== vcs_arguments
[origin_projects.vcs_data, origin_projects.vcs_installation_id]

Sometimes this is a triple, but only in old stuff (2017-10 or so)
Sometimes has a NULL, again only in old stuff, maybe

=== net_error_code
=== net_error_msg
Legacy from microservice days... 1004 wk:run:build

=== scheduler_sync
=== sync_count
TODO: seems to be used to track polling of state, but not sure how...

=== package_ident
Contains fully qualified name of package built, or null when failed

Interesting note; we see a lot failed jobs with package_ident field filled.
Somehow the job worker is sending back the package name. Is this happening early... maybe capturing pre_build? 

=== archived
Log has completed streaming from worker and has been archived wherever they go (S3/local store)


=== channel
Channel to upload the built package to
Channel name 'bldr-{owner_id}'
=== worker
TODO What does it mean?

=== target


== busy_workers

                                          Table "public.busy_workers"
   Column    |           Type           |          Modifiers           | Storage  | Stats target | Description 
-------------+--------------------------+------------------------------+----------+--------------+-------------
 ident       | text                     |                              | extended |              | 
 job_id      | bigint                   |                              | plain    |              | 
 quarantined | boolean                  |                              | plain    |              | 
 created_at  | timestamp with time zone | default now()                | plain    |              | 
 updated_at  | timestamp with time zone | default now()                | plain    |              | 
 target      | text                     | default 'x86_64-linux'::text | extended |              | 
Indexes:
    "busy_workers_ident_job_id_key" UNIQUE CONSTRAINT, btree (ident, job_id)
