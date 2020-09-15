
# basic loop

Scheduler will be an event driven loop

Message sources:

* API: (new build jobs)
* WM: Worker manager: (worker availability/results)
* LM: Maybe log manager (do we have a separate state to track completion of
logs? how do we do that?

Potential events are:

* New group added (API)

  Notify workers for target
select id, group_id, job_state, manifest_ident, dependencies, array_length(dependencies, 1) as count from job_graph where array_length(dependencies, 1) is null;
* Cancellation of group (API)

* Worker x wants job for target t (WM)

* Worker completes job #n with state success/fail (WM)
    * On success: Need to identify/update what jobs are now Ready
    * On fail: 	Need to mark any dependent jobs as dependency failed (recursively!)
    * Need to determine if group is complete (no jobs are in flight or startable)
      (Could be separate event, or not)

    * If available work for target t, notify worker manager to look
      for workers for target t. Risk of fanout here; need to think
      about that.
	  May end up pulling new groups into dispatching depending on how
      we want to manage things.

* Worker goes away (or any other retryable error) (WM)

  Job moved from Running back to Ready.

* Logs succesfully committed to storage (LM)
  This may not actually work this way; we may not mark the job
  complete and the worker free until the log is streamed/flushed to
  S3. IF that is true, then all the scheduler needs to know is when
  the job is really done. Alternately we could split things up, where
  we mark the job complete.

  What is the guarantee around keeping logs? Do we need to fail the
  job if the logs are dropped (I'd argue no)

# what could go wrong

Queues get cross blocked (try to send when queue is full, blocking us)
Use care with message amplification. The 'jobs available' message is a
risk point.

Dropped messages:
* Loosing job completion would cause system to stall. Maybe we need
  some sort of watchdog to make sure that we either check for
  completed jobs or look for lack of forward progress of group

INVARIANT: A Queued group should either have
  1) no available workers for its arch
  2) or it needs to be making progress; e.g something running.
  3) or it needs to be completed.

  Check counts of it's job states (x in Ready, y in Complete, etc) and
  if it hasn't changed in some time, Cancel and log the living
  daylights out if it at first, be clever later.
  

# Scheduler: updating job dependencies

The difference between a WaitingOnDependency and a Ready job is whether the dependencies are fullfilled; we
mark that by changing the state.

The database entry has an array field containing the IDs of it's dependencies. The trick is to track and
update Ready fields when a dependency is completed.

A few approaches suggest themselves

## Option 1: Smart query

We could just mark jobs complete, and the build a query with a subselect that finds all job entries that
have all their dependencies in the Complete state; that query could be used to update to Ready.

That would be a complex query, and to keep it tractable we'd probably want to find ways to limit the
number of rows examined. (filtering by state WaitingOnDependency, and possibly only updating a single job
group). It also would most likely require some care to keep correct. We'd need an (partial) index over the
dependency array, but it wouldn't be updated except when groups were added or deleted.

It also has the potential to be slow.

The advantage is that it would have a simple recovery path. Marking a job complete either succeeds or
fails, and the computation to find jobs to update to Ready is stateless and idempotent.

We probably will want a query like this in any event, simply to allow us a debugging dashboard and
recovery path when things go wrong.

## Option 2: Modify dependency array

A second option would be to build a transaction that simultaneously marks a job complete, and deletes
itself from any dependency arrays that reference it. We could then select jobs that have no dependencies
and update them to Ready. (Note: that we'd need that query no matter what, since we will have to figure
out where to start a group)

Deletion could be an expensive task, and in particular create a lot of garbage. For debugging, we
probably would want to clone the dependency array (unfulfilled dependencies or the like). We'd need an
(partial) index over that, which would be heavily updated as deletions proceeded.

This is relatively simple to write, and provides an easy to track indication of where each job was
waiting on dependencies.

## Option 3: Counter

The second option could be refined with a simple counter 'unfulfilled\_dependency\_count'. On job
completion we'd find every job that had it as a dependency, and decrement the counter. These would have
to be done as a single transaction for safety. We'd then have a separate update to find jobs with a zero
Ready state, and select WaitingOnDependency and 0 unfullfiled.

We'd be updating the index with the counter pretty frequently, so there would be some cost there. 

This doesn't give us visibility into what precisely is outstanding. 
