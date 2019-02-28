#!/bin/bash

# This script installs baseline packages for Linux2.
# It does *NOT* use a bootstrap bundle, and hence depends
# on the production Depot being up.

set -euo pipefail

########################################################################
# Preliminaries, Helpers, Constants

self=${0}
log() {
  >&2 echo "${self}: $1"
}

find_if_exists() {
    command -v "${1}" || { log "Required utility '${1}' cannot be found!  Aborting."; exit 1; }
}

# These are the key utilities this script uses. If any are not present
# on the machine, the script will exit.
curl=$(find_if_exists curl)

# Builder services
services_to_install=(builder-worker)

# We're always going to need all the packages for running the
# Supervisor.
sup_packages=(hab-launcher
              hab
              hab-sup)

# First, install hab with a Linux2 target
curl https://raw.githubusercontent.com/habitat-sh/habitat/master/components/hab/install.sh | sudo bash -s -- -t x86_64-linux-kernel2

# Install supervisor
for pkg in "${sup_packages[@]}"
do
    pkg_name=${pkg##core/} # strip "core/" if it's there
    hab pkg install core/"${pkg_name}"
done

# Install builder packages
for pkg in "${services_to_install[@]}"
do
    pkg_name=${pkg##habitat/} # strip "core/" if it's there
    hab pkg install habitat/"${pkg_name}"
done

# Now we ensure that the hab binary being used on the system is the
# one that we just installed.
#
# TODO fn: The updated binlink behavior is to skip targets that already exist
# so we want to use the `--force` flag. Unfortunetly, old versions of `hab`
# don't have this flag. For now, we'll run with the new flag and fall back to
# running the older behavior. This can be removed at a future date when we no
# lnger are worrying about Habitat versions 0.33.2 and older. (2017-09-29)
hab pkg binlink core/hab hab --force \
  || hab pkg binlink core/hab hab

# Install docker via apt-get for now until we hammer out the
# steps with the hab package
apt-get update
apt-get -y install docker.io
