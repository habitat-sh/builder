#!/bin/bash

set -eu

echo "--- Generating signing key"
hab origin key generate "$HAB_ORIGIN"

# Using the dev studio is very fragile so we get to replicate some of the 
# functionality to run our api tests in CI. Yay.
# TODO: Maybe this belongs after a merge, but before artifacts are deployed 
# to acceptance?


# Build the packages we want to use
# TODO: Take advantage of sccache to improve build times
# TODO: bldr.toml gives us an idea of what changes can trigger a package build. 
#       use it to inform and improve build times.

echo "--- Building changed builder components"
mkdir -p logs
for component in api jobsrv worker; do
    echo "--- Building builder-$component"
    echo "Redirecting log output; See build artifact 'builder-$component.build.log'"
    hab pkg build components/builder-"$component" > logs/builder-"$component".build.log
    # Install the thing we just built, so that we can load it later.
    ( 
        source results/last_build.env
        # shellcheck disable=SC2154
        hab pkg install results/"$pkg_artifact"
    )
done

useradd hab --user-group --no-create-home

mkdir -p /hab/sup/default
# TODO: The sup/svc loading config work that is in flight would be useful here
env HAB_FUNC_TEST=1 \
    HAB_NONINTERACTIVE=true \
    HAB_NOCOLORING=true \
    hab sup run --no-color  > /hab/sup/default/sup.log 2>&1 & 

for retry in $(seq 1 30); do 
  sleep 2
  if hab sup status >/dev/null 2>&1; then
    break
  fi
  echo "Supervisor not started: $retry"

  if [ "$retry" -eq 5 ]; then
    echo "Supervisor not started after 60 seconds"
    exit 1
  fi
done

echo "--- Loading minio,memcached, and datastore services"
hab svc load "$HAB_ORIGIN"/builder-minio
hab svc load "$HAB_ORIGIN"/builder-memcached
hab svc load "$HAB_ORIGIN"/builder-datastore

echo "--- Waiting for the datastore service generate a password"
for retry in $(seq 1 30); do 
  sleep 2
  if [ -f /hab/svc/builder-datastore/config/pwfile ]; then
    break
  fi
  echo "Datastore pwfile not available yet: $retry"

  if [ "$retry" -eq 30 ]; then
    echo "Datastore failed to generate pwfile after 60 seconds"
    hab svc status
    exit 1
  fi
done

echo "--- Writing db password to config"
PGPASSWORD="$(< /hab/svc/builder-datastore/config/pwfile)"
cat << EOC > datastore.toml
[datastore]
password="$PGPASSWORD"
EOC

echo "--- Applying configuration to jobsrv and api"
hab config apply builder-jobsrv.default "$(date +%s)" datastore.toml
hab config apply builder-api.default "$(date +%s)" datastore.toml

echo "--- Creating empty builder-github-app.pem"
mkdir -p /hab/svc/builder-api/files
mkdir -p /hab/svc/builder-worker/files
touch /hab/svc/builder-api/files/builder-github-app.pem
touch /hab/svc/builder-worker/files/builder-github-app.pem

echo "--- Loading api,proxy,jobsrv and worker services"
hab svc load "$HAB_ORIGIN"/builder-api \
    --bind memcached:builder-memcached.default \
    --bind jobsrv:builder-jobsrv.default
hab svc load "$HAB_ORIGIN"/builder-api-proxy \
    --bind http:builder-api.default

hab svc load "$HAB_ORIGIN"/builder-jobsrv
hab svc load "$HAB_ORIGIN"/builder-worker \
    --bind jobsrv:builder-jobsrv.default \
    --bind depot:builder-api-proxy.default

echo "--- Generating bldr user keys"
KEY_NAME=$(hab user key generate bldr | grep -Po "bldr-\\d+")

# API will often fail recieve the first key upload. Wrap all the services
# in a small amount of retries to ensure the system is healthy before 
# running tests.
for svc in api jobsrv worker; do
    echo "--- Uploading bldr user keys to $svc"
    for retry in $(seq 1 5); do
        hab file upload "builder-${svc}.default" "$(date +%s)" "/hab/cache/keys/${KEY_NAME}.pub"
        hab file upload "builder-${svc}.default" "$(date +%s)" "/hab/cache/keys/${KEY_NAME}.box.key"
        # The service will be in a panic/boot loop before upload, take a rest before checking the status
        sleep 5
        state="$(hab svc status | grep "builder-$svc.default" |awk '{print $4}')"
        if [ "$state" == "up" ]; then
            break
        fi
        if [ "$retry" -eq 5 ]; then
            echo "$svc failed to start after 5 tries"
            hab svc status
            exit 1
        fi
    done
done

echo "--- Listing services under test"
hab svc status

echo "--- Running tests"
# npm is installed via Habitat and is provided as part of our CI container image
( 
    cd test/builder-api
    # Install it globally so we can use `mocha` rather than `npm run mocha`
    #
    # The observed behavior of `npm run mocha` is equivalent to `mocha --bail` 
    # which halts execution after the first failure. We're running this in CI 
    # so it's nice to know everything that failed rather than just the first thing
    npm install mocha --global
    mocha src --retries 5
)
