# Implementation of build manifest based builds

We have recently extended our graph data structure to be able to handle cyclic dependencies. These arise when
packages have interdependent build relationships; for example you need gcc to build make, but make might be
required to build gcc. This has produced a new representation of the work required for a build; a build
manifest. This is a DAG with all the inter-package dependencies completely specified, and build cycles
unrolled to make sure we converge those cyclic dependencies. Packages may occur multiple times, as we may
first rebuild make with the old gcc, use that to make the new gcc, and then rebuild make again.

The existing scheduler is unable to handle this, and in fact doesn’t actually use the existing build graph in places. It is also devoid of tests, and is implemented in a way that is very difficult to test. Initially we wanted to reuse/adapt it, but after some study we have concluded that the scheduler and associated database structures need a complete rewrite.
# High level view
## Design goals
- Front end API remains unmodified
- Job workers remain unmodified
- Upgrade preserves history of old builds
- Downgrade lets you see old jobs, (but not new ones)
- As much as possible the new jobserv should be able to run in ephemeral instances (unique internal state kept in sql, with ability to recover). Logs might be a problem, as they are accumulated locally, and we’re not going to tackle that today. 
- Eye kept towards future HA implementation
- Design for easy testability. 
- Design for observability and determinism. 
- Designed for restartability and use of partial work
- Minimize use of ZMQ internally (external proto is ZMQ based and will not change)

Code changes:
- scheduler.rs rewritten from scratch
- worker_manager.rs modified to understand new job type
- handlers.rs modified to handle new job type and changes to group

Schema changes:
- groups schema extended with version and either other extra fields or an additional table
- group_projects replaced
- jobs table extended, but kept as similar as possible to keep worker_manager untouched.
- Replace user-presented job/group ids with guids, or some other value that isn’t a row id. (does this alter our API contract with the FE?)

New flow
 The manifest is rendered into the database, either as a serialized graph structure (json?) or directly as rows in a table.The basic lifecycle is 
- Queued: no package builds have started
- Dispatching: package builds are in flight
- Complete: all package builds are complete w/o error
- Failed: some packages failed to build
- Canceled: execution was terminated externally

Note: we may want to limit the number of groups in Dispatching, or otherwise create a process to prioritize finishing jobs whenever possible. 
Note: we may want to distinguish in flight cancellation from completed cancellation

Notes: projects may become unbuildable at any time, and we will need to recognize and handle that (plan disconnection, git access denied)

# Group Lifecycle

When we first receive a trigger event, we have a set of packages (base or kernel) that are modified
(Triggered?). The graph is used to compute the rebuild manifest from the kernel (Created). We then expand the
manifest into jobs (Queued). At some point it becomes eligible to be started, and its jobs now become
available to work on (Dispatching?  Dispatchable) . When all jobs are successfully completed it is marked
Completed.

It might have a fatal failure, and need to end (do we make a best effort to do as much work as possible first?
Or fail fast). During that cleanup it moves to FailureCleanup. Once that is done, it moves to Failed.

At any point before completion or failure it might be canceled, and
move to a cleanup phase (Cancelling) and when that is done, it is
marked Canceled.

Triggered: trigger event and kernel of packages known
- It is moved to Created once the manifest is computed and stored
- It may fail if the manifest is problematic (->Failed)
- It may be canceled (->Canceled) (unlikely because of timing)
Created: build manifest is known and stored
- It is moved to Queued once all the jobs are created.
- It may fail if (but why?) (->Failed)
- It may be canceled (->Canceled) (unlikely because of timing)
Queued: Jobs are created, but no work started
- it is moved to Dispatching when we need new work (this might be immediate, or as a pull when we run the job queue dry). It may be worthwhile minimizing the groups in flight to improve visibility and make it easier to track what’s going on.
- It may be canceled (->Canceled)
Dispatching: package builds are in flight
- It is moved to Complete when all jobs are finished or failed.
- It may fail if one or more of the jobs fail (->Failed or FailureCleanup) We should make a best effort to do as much as we can before failing
- It may be canceled (->Canceled)

Complete: all package builds are complete w/o error. Terminal state. Q: What work needs to be done on completion?
FailureCleanup: a fatal failure has happened, and we’re cleaning things up (waiting for workers to finish).
- Once done go to Failed.
Failed: some packages failed to build. Terminal state. 
Canceling: execution is being terminated, but we’re waiting for workers to finish.Once done move to Canceled.
Canceled: execution was terminated externally. Terminal state

The current concept for implementation is to record each state transition in the database, and make the transitions as restartable as possible. This minimizes the potential loss on failure.

Open question: do we try to extend the existing groups table, or do we create a new one and have parallel code. How do we want to handle retention, back compat and rollback with this?
If we create a new table, we have to have the code smart enough to handle that. If we reuse the old table we’re constrained on what we put in group_state to remain back compat. That makes adding new states difficult; we might need to have a second column for the new, and back-map into the old as an approximation.

    Groups table
    id: sequence
    group_state: (do we just leave this alone, or do we have an augmented enum with all of the states)
    group_version: (0 = old, 1= new)
    created_at, updated_at
    target
    new_group_state: (maybe)
    manifest: JSONB
    trigger: if the manifest doesn’t contain this info



Differences from old system
Note: the existing code has Queued and Pending, should figure out the distinction

# Job schedule lifecycle

In the current system when a group is first created, the group and the group projects entries are created for every package that is
being built as part of the group (see job\_group\_create, jobsrv::server::handlers#302). The job entries are
created when the group is scheduled (see schedule\_job,jobsrv::server::scheduler#533). The dependencies between
the jobs aren’t represented in the job entries, but are instead looked up separately from the origin_packages
table. Note: The Processing state doesn't seem to be used; things move from Pending to Dispatched directly.

The new system will do things differently. First, the manifest contains the complete dependency
relationships between the packages being rebuilt, and that information needs to be represented in the
job_dependencies table. This is necessary because the new system will build packages multiple times, and
the actual information is unique to the manifest in question.



Jobs entries are created in the state Pending as a when a group moves
from Created to Queued. When a group is moved to Dispatching, all of
the jobs in the group are moved to Schedulable. When an Schedulable job has
all of its dependencies built it is marked Eligible.  When a worker
becomes available, we choose the highest priority Eligible job and
mark it Dispatched.

There are a few outcomes as the worker builds the job. The job can
complete successfully, look for jobs that now have all their
dependencies complete, and mark those as Eligible and move to
Built.

The job can fail for some reason, mark dependent jobs as DependencyFailed and move to JobFailed. The
worker can die/go silent, and move to JobLost. (We might want to retry JobLost). JobLost might not be a necessary state; we might just return it to Eligible. 

The group might be cancelled, and move to CancelPending. Once the worker actually verifies that it’s
canceled, it is marked CancelComplete.

Schedule priority. The initial prioritization scheme will be oldest by creation date first. We will want a more sophisticated scheme eventually (some initial research suggests that prioritising jobs that ‘unlock’ many other jobs is very worthwhile for maximum parallelism), but that won’t be done immediately. It is worthwhile having a simple prioritization scheme to provide some determinism to our queuing algorithm, both for testing and better user experience. 

To keep things easily observable, we should keep as few as possible groups active while keeping the workers busy. We should prioritize jobs in older groups before those in newer ones. New groups will only be pulled in if workers are inactive (perhaps even after a bit of a delay). When a new group is marked 

Schema note:
id: sequence 
group_id: FK to groups table

job_state: State as above

remaining_dependency_count: count of unfinished deps
dependencies: ARRAY of ids (list of things we depend on)

created_at
updated_at

Build dispatch info:  
project_name: name
project_build_data: (plan path, etc) MAYBE LATE BIND IN WORKER INSTEAD
vcs data: vcs system type, provider info (github url, access id) MAYBE LATE BIND IN WORKER INSTEAD

channel:

Build info (maybe parallel table)
worker_id:
worker_info: (maybe log more detailed info about the build worker)
build_info: (JSOB with interesting things like git sha, etc artifact checksums)
build_started_at
build_finished_at
error_code:
error_msg:
log_archived
built_package_ident



Selecting next job
Must be Schedulable.
Worker capable: target of the worker (future might need an origin match as well)
Pick highest priority (earliest created_at)

Finding newly readied jobs from completed job:
as transaction do both:
UPDATE WHERE dependencies contains completed_job_id and decrement remaining_dependency_count by 1
UPDATE job_state to schedulable if Eligible and remaining_dependency_count is 0

Do we have a check to find jobs that have Schedulable jobs but no Eligible (or schedulable with 0 remaining deps)

Refinements: matching worker by origin (for bring your own worker). Priority scheme for scheduling jobs.
TODO: discuss splitting package builds from container builds, but not in this round of work, because that changes the worker contract. 

NOTE: Either package or job result metadata should have details about it’s build host and config.

NOTE figure out how we are going to handle deconflicting the various versions of packages in cycles. Answer: at some point add a dependency edge on cyclic built package ‘p’, so that p(n+1) depends on every package that depends on v(n); that ensures that p(n+1) is not built and uploaded into the channel until everyone who needed p(n) is finished. 

TODO Figure out failure cases and watchdog

# Implementation

## Management of available jobs
Logically a job becomes available to execute when it has no dependencies

## Next job selecton


# Misc topics

## Debugging
Look into the following libraries for debugging support

Needs vector clock support
https://bestchai.bitbucket.io/shiviz/
https://distributedclocks.github.io/


https://github.com/open-telemetry/opentelemetry-rust
https://github.com/cuviper/rust-libprobe
https://github.com/redsift/ingraind/
https://github.com/redsift/redbpf

