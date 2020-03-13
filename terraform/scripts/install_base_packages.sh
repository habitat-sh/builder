#!/bin/bash

# Downloads the latest builder bootstrap tarball from S3 and installs
# the desired packages into /hab/cache/artifacts. All
# Supervisor-related packages are always installed; additional
# packages from the archive can be installed by specifying them as
# arguments. See usage_message below.

set -euo pipefail

# default, can be overridden with -t argument
pkg_target="x86_64-linux"
declare -a services_to_install='()'

export pkg_target services_to_install

########################################################################
# Preliminaries, Helpers, Constants

self=${0}
log() {
  >&2 echo "${self}: $1"
}

find_if_exists() {
    command -v "${1}" || { log "Required utility '${1}' cannot be found!  Aborting."; exit 1; }
}


usage_message="This script installs base Habitat package artifacts from a bootstrap bundle.
USAGE:
    $0 [FLAGS]

FLAGS:
    -h          Prints help information
    -s          Service name to install in addition to base artifacts. If multiple, use -s for each.
                (example: $0 -t x86_64-linux -s habitat/builder-api -s habitat/builder-api-proxy)
    -t          Platform target type. (values: x86_64-linux, x86_64-linux-kernel2 or x86_64-windows) [default: x86_64-linux ]
                (example: $0 -t x86_64-linux)
"

usage() {
  echo "${usage_message}" >&2
}

exit_abnormal() {
  usage
  exit 1
}

while getopts ":ht:s:" option; do
  case "${option}" in
    h)
      usage
      exit
      ;;
    s)
      services_to_install+=("${OPTARG}")
      ;;
    t)
      pkg_target=${OPTARG}
      ;;
    :)
      echo "Error: -${OPTARG} requires an argument." >&2
      exit_abnormal
      ;;
   \?)
      echo "Invalid Option: -${OPTARG}" >&2
      exit_abnormal
      ;;
  esac
done

# These are the key utilities this script uses. If any are not present
# on the machine, the script will exit.
awk=$(find_if_exists awk)
curl=$(find_if_exists curl)
shasum=$(find_if_exists shasum)
tar=$(find_if_exists tar)

# This is where we ultimately put all the things. All contents of the
# bucket will be publicly readable, so we can just use curl to grab them.
s3_root_url="https://s3-us-west-2.amazonaws.com/habitat-builder-bootstrap"

# We're always going to need all the packages for running the
# Supervisor.
sup_packages=(hab-launcher
              hab
              hab-sup)

# Helper for syslog logging
helper_packages=(nmap)

########################################################################
# Download bootstrap archive from S3

# Pull down the most recent tarball manifest file from S3. The name of
# the corresponding tarball is the first line of the file.
manifest_url=${s3_root_url}/LATEST
log "Downloading latest builder tarball manifest from ${manifest_url}"
${curl} --remote-name ${manifest_url} >&2
latest_archive=$(${awk} 'NR==1' LATEST)

# Now that we know the latest tarball, let's download it, too.
latest_package_url=${s3_root_url}/${latest_archive}
log "Downloading ${latest_archive} from ${latest_package_url}"
${curl} --remote-name ${s3_root_url}/"${latest_archive}" >&2

# Verify the tarball; the SHA256 checksum is the 2nd line of the
# manifest file.
checksum=$(${awk} 'NR==2' LATEST)
log "Verifying ${latest_archive} with checksum ${checksum}"
${shasum} --algorithm 256 --check <<< "${checksum}  ${latest_archive}" >&2

########################################################################
# Unpack the archive

tmpdir=hab_bootstrap_$(date +%s)
mkdir -p "${tmpdir}"

${tar} --extract \
       --verbose \
       --file="${latest_archive}" \
       --directory="${tmpdir}"

# This is the hab binary from the bootstrap bundle. We'll use this to
# install everything.
hab_bootstrap_bin="${tmpdir}/bin/hab-${pkg_target}"

########################################################################
# Install the desired packages
#
# Note that this only puts the packages into /hab/cache/artifacts; it
# does not run `hab svc load`. We'll want to do that later, to ensure
# that the Supervisor running in the proper environment (e.g., under
# systemd, and not this script).

# Install the key(s) first. These need to be in place before
# installing any packages; otherwise, hab will try to contact a depot
# to retrieve them to verify the packages.
log "Installing public origin keys"
mkdir -p /hab/cache/keys
cp "${tmpdir}"/keys/* /hab/cache/keys

# When installing packages (even from a .hart file), we pull
# dependencies from Builder, but we pull them *through the artifact
# cache*. If we put all the hart files in the cache first, we should
# be able to install everything we need. There will be some extra
# artifacts, but that's a minor concern.
log "Populating artifact cache"
mkdir -p /hab/cache/artifacts
cp "${tmpdir}"/artifacts/* /hab/cache/artifacts

for pkg in "${sup_packages[@]}" "${helper_packages[@]}"
do
    if [[ -n "${pkg}" ]]; then
      pkg_name=${pkg##core/} # strip "core/" if it's there
      shopt -s nullglob
      # we want globbing here
      # shellcheck disable=SC2206
      hart_paths=(${tmpdir}/artifacts/core-${pkg_name}-*-${pkg_target}.hart)
      if [[ "${#hart_paths[@]}" -gt 0 ]]; then
        for hart_file in "${hart_paths[@]}"
        do
          # Using a fake depot URL keeps us honest; this will fail loudly if
          # we need to go off the box to get *anything*
          HAB_LICENSE="accept" \
          HAB_BLDR_URL=http://not-a-real-depot.habitat.sh \
                 ${hab_bootstrap_bin} pkg install "${hart_file}"
        done
      fi
      shopt -u nullglob
    fi
done

for pkg in "${services_to_install[@]:-}"
do
    if [[ -n "${pkg}" ]]; then
      pkg_name=${pkg##habitat/} # strip "core/" if it's there
      shopt -s nullglob
      # we want globbing here
      # shellcheck disable=SC2206
      hart_paths=(${tmpdir}/artifacts/core-${pkg_name}-*-${pkg_target}.hart)
      if [[ "${#hart_paths[@]}" -gt 0 ]]; then
        for hart_file in "${hart_paths[@]}"
        do
          # Using a fake depot URL keeps us honest; this will fail loudly if
          # we need to go off the box to get *anything*
          HAB_LICENSE="accept" \
          HAB_BLDR_URL=http://not-a-real-depot.habitat.sh \
                 ${hab_bootstrap_bin} pkg install "${hart_file}"
        done
      fi
      shopt -u nullglob
    fi
done

# Now we ensure that the hab binary being used on the system is the
# one that we just installed.
#
# TODO fn: The updated binlink behavior is to skip targets that already exist
# so we want to use the `--force` flag. Unfortunetly, old versions of `hab`
# don't have this flag. For now, we'll run with the new flag and fall back to
# running the older behavior. This can be removed at a future date when we no
# lnger are worrying about Habitat versions 0.33.2 and older. (2017-09-29)
${hab_bootstrap_bin} pkg binlink core/hab hab --force \
  || ${hab_bootstrap_bin} pkg binlink core/hab hab
if ${hab_bootstrap_bin} pkg path core/nmap 2>/dev/null; then
  ${hab_bootstrap_bin} pkg binlink core/nmap ncat --force \
    || ${hab_bootstrap_bin} pkg binlink core/nmap ncat
fi
