
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

* Cancellation of group (API)

* Worker x wants job for target t (WM)

* Worker completes job #n with state success/fail (WM)
    * On success: Need to identify/update what jobs are now Eligible
    * On fail: 	Need to mark any dependent jobs as dependency failed (recursively!)
    * Need to determine if group is complete (no jobs are in flight or startable)
      (Could be separate event, or not)

    * If available work for target t, notify worker manager to look
      for workers for target t. Risk of fanout here; need to think
      about that.
	  May end up pulling new groups into dispatching depending on how
      we want to manage things.

* Worker goes away (or any other retryable error) (WM)

  Job moved from Dispatched back to Eligible.

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
  2) or it needs to be making progress; e.g something dispatched.
  3) or it needs to be completed.

  Check counts of it's job states (x in eligible, y in built, etc) and
  if it hasn't changed in some time, Cancel and log the living
  daylights out if it at first, be clever later.
  


* 


