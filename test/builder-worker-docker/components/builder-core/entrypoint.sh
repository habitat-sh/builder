#!/bin/bash

export HAB_ORIGIN=habitat
export CHANNEL=stable

source ./bldr.env
source ./functions.sh

set -m

#  Generate a builder key.
hab origin key generate --cache-key-path /hab/cache/keys habitat
sleep 1

#  Build the components.
echo "Building worker" && mkdir -p /src && cd /src && git clone --single-branch --branch main https://github.com/habitat-sh/builder.git

echo "Building worker with hab pkg build" && cd /src/builder && hab pkg build components/builder-worker
echo "Builder worker up to date"

HAB_FUNC_TEST=1 hab sup run -v &

cd /
echo "Working directory: "
echo `pwd`

echo "........ waiting a few seconds to start ......"
sleep 10
init_datastore


#  Build the components.
#echo "Building worker" && hab-studio build components/builder-worker
#echo "Builder worker up to date"

sleep 5
start_bldr_services

cp ./builder-github-app.pem /hab/svc/builder-worker/files
echo "DONE STARTING SERVICES"

#  Try to build the builder


fg %1
