# Builder API Functional tests

## What are these tests?

These are end-to-end functional tests for the builder-api.

## How to run these tests

If you're already in a studio with a Supervisor running

```shell
[1][default:/src:0]# sup-term
```

Otherwise, enter a studio with no supervisor with `HAB_STUDIO_SUP=false hab studio enter`
Also, it can sometimes be a good idea to `hab studio rm` sometimes.

### Start a supervisor in test mode

```shell
[1][default:/src:0]# HAB_FUNC_TEST=1 sup-run
```

## Now that the Supervisor is running

```shell
[2][default:/src:0]# hab sup status
```

If not already running, launch services and wait for them to come up:

```shell
[3][default:/src:0]# start-builder
```

When everything is ready, `hab sup status` will look like:

```shell
package                                        type        desired  state  elapsed (s)  pid    group
habitat/builder-api/10315/20240913162802       standalone  up       up     88           45379  builder-api.default
habitat/builder-api-proxy/9639/20240722052815  standalone  up       up     87           45385  builder-api-proxy.default
habitat/builder-datastore/7795/20181018210336  standalone  up       up     121          45215  builder-datastore.default
habitat/builder-minio/7764/20181006010221      standalone  up       up     74           45399  builder-minio.default
habitat/builder-memcached/9467/20220628111248  standalone  up       up     113          45318  builder-memcached.default
core/sccache/0.8.1/20241018040537              standalone  up       up     118          45272  sccache.default
```

If we do not build, we'd be testing against the stable package versions of the builder components:

```shell
[4][default:/src:0]# build-builder
[5][default:/src:0]# test/builder-api/test.sh
```

Keep in mind that since you are developing that you may need to customize the execution environment.  Environment variables of interest include, but are necessarily limited to, the following ones that control the channels used.

```shell
```

Then, repeat as necessary. The following condenses the above information in the easiest happy path for ease of reference.

```shell

# enter a studio with no supervisor with 
$ hab studio rm # Do a studio rm "as feels right".  Use hab studio rm as you would "make clean".
$ HAB_STUDIO_SUP=false hab studio enter
# ...studio starts...

[1][default:/src:0]# HAB_FUNC_TEST=1 sup-run
# Load any environment variables you need now
[3][default:/src:0]# start-builder
[4][default:/src:0]# hab svc status

# make changes to the code base
[5][default:/src:0]# build-builder
[6][default:/src:0]# test/builder-api/test.sh

# and now you're in a development loop: change, build, test
```
