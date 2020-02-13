#!/bin/bash

# Create a tar file of all the Habitat artifacts needed to produce a
# functioning Builder installation and upload it to S3. This includes
# *all* dependencies. The goal is to have everything needed to run the
# Supervisor and all Builder services *without* needing to talk to a
# running Builder.
#
# Because you have to bootstrap yourself from *somewhere* :)
#
# This script uploads the tar file to S3, so it will need AWS
# credentials. We use the standard AWS CLI for this, so any
# environment variables or configuration files that it recognizes,
# this script will also recognize. It uses our "habitat" profile, so
# just make sure your credentials line up with that.
#
# It also uploads a "manifest" file, which is just a text file
# containing the name of the tar file, its checksum, and the contents
# of the archive. This manifest file will also be uploaded into
# S3. Additionally, the file `LATEST` in the S3 bucket will always be
# a copy of the most recent manifest. This can be used as a "pointer"
# to find the latest tar artifact.
#
# This generates a tar file (not tar.gz; further compression doesn't
# remove much, given that everything inside is already compressed
# anyway) that has the following internal structure:
#
# |-- ARCHIVE_ROOT
# |   |-- artifacts
# |   |   `-- all the hart files
# |   |-- bin
# |   |   `-- hab
# |   `-- keys
# |       `-- all the origin keys
#

set -euo pipefail

log() {
  >&2 echo "BOOTSTRAP LOG: $1"
}

# This bit of magic strips off the Habitat header (first 6 lines) from
# the compressed tar file that is a core/hab .hart, and extracts the
# contents of the `bin` directory only, into the ${dir}
# directory.
#
# Note that `dir` should exist before calling this function.
extract_hab_binaries_from_hart() {
    local hart="${1}"
    local dir="${2}"

    tail --lines=+6 "${hart}" | \
        tar --extract \
            --directory="${dir}" \
            --xz \
            --strip-components=7 \
            --wildcards "hab/pkgs/core/hab/*/*/bin/"
}

# Helper function for running s3 cp with appropriate settings.
s3_cp() {
    local src="${1}"
    local dest="${2}"
    aws --profile habitat \
        s3 cp "${src}" "${dest}" \
        --acl public-read
}

# This is where we ultimately put all the things in S3.
readonly s3_bucket="habitat-builder-bootstrap"

# This will be form the name of the archive and manifest files, as
# well as the directory into which the hart files are downloaded.
#
# NOTE: Do NOT alter this value to have "-" characters; see below with
# the `hab_hart` variable.
readonly this_bootstrap_bundle="hab_builder_bootstrap_$(date +%Y%m%d%H%M%S)"

########################################################################

sandbox_dir="${this_bootstrap_bundle}"
log "Downloading packages into ${sandbox_dir}"
mkdir "${sandbox_dir}"

hab pkg download \
    --download-directory="${sandbox_dir}" \
    --file=".expeditor/builder_seed.toml" \

########################################################################

# Since the whole point of this archive is to have something
# self-contained from which to bootstrap an entirely new Builder
# environment, we'll also need access to a `hab` binary.
#
# Conveniently, we have just downloaded a hart file for that. To make
# things easy on us, we can extract the stand-alone `hab` binary from
# that hart file (this assumes Linux packages, naturally) and store it
# at `bin/hab` in the tar archive.

# The regex is to add the most general placeholders for "version" and
# "release" in the hart name, while also avoiding similarly-named
# packages like
# `core-hab-launcher-${VERSION}-${RELEASE}-x86_64-linux.hart`, etc.
hab_hart=$(
    # Until habitat/builder-worker is part of our release pipeline,
    # it's likely that we will end up with more than one `core/hab`
    # artifact: one from the release, and the previous one, brought in
    # by habitat/builder-worker (because, until that's automated, it
    # won't have been updated to depend on the new one yet!)
    #
    # Once this is no longer the case, you can remove the warning
    # comment at the declaration of `${this_bootstrap_bundle}`.
    #
    # The `sort` invocation will sort the output of `find` by the
    # release timestamp, and then take the most recent one (i.e., the
    # last)
    #
    # For example:
    #
    # Field 1 (using "-" as a separator)                 ...2   ...3           ...4
    # ---------------------------------------------------V---V------V--------------V
    # hab_builder_bootstrap_20191122182751/artifacts/core-hab-0.90.6-20191112141314-x86_64-linux.hart
    find "${sandbox_dir}/artifacts" -type f -regex '.*/core-hab-[^-]*-[^-]*-x86_64-linux.hart' \
        | sort --field-separator="-" --key=4 --numeric-sort \
        | tail -n1
)
log "Extracting hab binary from ${hab_hart}"
bin_dir="${sandbox_dir}/bin"
mkdir -p "${bin_dir}"
extract_hab_binaries_from_hart "${hab_hart}" "${bin_dir}"

########################################################################

archive_name="${this_bootstrap_bundle}.tar"
log "Generating bootstrap tar file: ${archive_name}"
tar --create \
    --verbose \
    --file="${archive_name}" \
    --directory="${sandbox_dir}" \
    artifacts keys bin

########################################################################

log "Checksumming bootstrap tar file"
checksum=$(sha256sum "${archive_name}" | awk '{print $1}')

########################################################################

manifest_file="${this_bootstrap_bundle}_manifest.txt"
log "Generating bootstrap manifest file: ${manifest_file}"
{
  echo "${archive_name}"
  echo "${checksum}"
  echo
  tar --list --file "${archive_name}" | sort
} > "${manifest_file}"

########################################################################

log "Uploading artifacts to S3 in ${s3_bucket}"
s3_cp "${archive_name}" "s3://${s3_bucket}"
s3_cp "${manifest_file}" "s3://${s3_bucket}"
s3_cp "s3://${s3_bucket}/${manifest_file}" "s3://${s3_bucket}/LATEST"
