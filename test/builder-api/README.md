# Builder API Functional tests

## What are these tests?

These are end-to-end functional tests for the builder-api.

## How to run these tests

If you're already in a studio with a Supervisor running

```
[1][default:/src:0]# sup-term
```

Otherwise, enter a studio with no supervisor

```
$ HAB_STUDIO_SUP=false hab studio enter
```

### Start a supervisor in test mode

```
[1][default:/src:0]# HAB_FUNC_TEST=1 sup-run
```

## Now that the Supervisor is running

```
[2][default:/src:0]# hab sup status
```

If not already running, launch services and wait for them to come up:
```
[3][default:/src:0]# start-builder
```
When everything is ready, `hab sup status` will look like:
```
package                                         type        state  uptime (s)  pid    group                       style
habitat/builder-sessionsrv/7049/20180208225209  standalone  up     22          45502  builder-sessionsrv.default  persistent
habitat/builder-router/7039/20180207225315      standalone  up     25          45293  builder-router.default      persistent
core/sccache/0.2.4/20180118212549               standalone  up     24          45298  sccache.default             persistent
habitat/builder-worker/7054/20180208233206      standalone  up     24          45321  builder-worker.default      persistent
habitat/builder-api/7052/20180208233114         standalone  up     24          45329  builder-api.default         persistent
habitat/builder-api-proxy/7052/20180208233113   standalone  up     24          45402  builder-api-proxy.default   persistent
habitat/builder-jobsrv/7049/20180208225208      standalone  up     21          45664  builder-jobsrv.default      persistent
habitat/builder-datastore/7043/20180208190943   standalone  up     24          45431  builder-datastore.default   persistent
habitat/builder-originsrv/7049/20180208225209   standalone  up     19          45827  builder-originsrv.default   persistent
```
If we do not build, we'd be testing against the stable package versions of the
builder components:
```
[4][default:/src:0]# build-builder
```
```
[5][default:/src:0]# test/builder-api/test.sh
```
Repeat as necessary