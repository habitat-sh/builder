#!/bin/bash

# The purpose of this script is to download the latest stable version of every
# package in the core-plans repo, tar them up, and upload to S3. It also supports
# downloading the archive from S3, extracting it, and uploading to a new depot.
#
# There are some environment variables you can set to control the behavior of this
# script:
#
# HAB_ON_PREM_BOOTSTRAP_BUCKET_NAME: This controls the name of the S3 bucket where
# the archive is placed. The default is habitat-on-prem-builder-bootstrap
#
# HAB_ON_PREM_BOOTSTRAP_S3_ROOT_URL: This controls the domain name for S3 where the
# files will be downloaded from. The default is https://s3-us-west-2.amazonaws.com
#
# HAB_ON_PREM_BOOTSTRAP_DONT_CLEAN_UP: This controls whether the script cleans up
# after itself by deleting the intermediate files that were created during its run.
# Setting this variable to any value will cause the cleanup to be skipped. By
# default, the script will clean up after itself.
#
# IT SHOULD BE NOTED THAT THIS SCRIPT DOES NOT WORK IN ITS CURRENT FORM. The key
# problem is that some packages that are the latest stable are built against libs
# that are older than the current stable version of those libs, so when you go to
# upload the resulting packages from the S3 archive, uploads fail because when
# trying to upload the transitive deps, the wrong versions are present (it's looking
# for older versions than what have been downloaded).

set -eo pipefail

help() {
    echo "Usage: on-prem-archive.sh {create-archive|populate-depot <DEPOT_URL>}"
}

exists() {
  if command -v $1 >/dev/null 2>&1
  then
    return 0
  else
    return 1
  fi
}

s3_cp() {
  aws s3 cp --acl=public-read ${1} ${2} >&2
}

check_tools() {
  local ref=$1[@]

  for tool in ${!ref}
  do
    if ! exists "$tool"; then
      echo "Please install $tool and run this script again."
      exit 1
    fi
  done
}

check_vars() {
  local ref=$1[@]

  for var in ${!ref}
  do
    if [ -z "${!var}" ]; then
      echo "Please ensure that $var is exported in your environment and run this script again."
      exit 1
    fi
  done
}

bucket="${HAB_ON_PREM_BOOTSTRAP_BUCKET_NAME:-habitat-on-prem-builder-bootstrap}"
marker="LATEST.tar.gz"

case "$1" in
  create-archive)
    required_tools=( aws git curl jq )
    required_vars=( AWS_ACCESS_KEY_ID AWS_SECRET_ACCESS_KEY )

    check_tools required_tools
    check_vars required_vars

    core_tmp=$(mktemp -d)
    core="$core_tmp/core-plans"
    upstream_depot="https://bldr.habitat.sh"
    bootstrap_file="on-prem-bootstrap-$(date +%Y%m%d%H%M%S).tar.gz"
    tar_file="/tmp/$bootstrap_file"
    tmp_dir=$(mktemp -d)

    git clone https://github.com/habitat-sh/core-plans.git "$core"
    pushd "$core"
    dir_list=$(find . -type f -name "plan.sh" -printf "%h\n" | sed -r "s|^\.\/||" | sort -u)
    total=$(echo "$dir_list" | wc -l)
    count="0"

    for p in $dir_list
    do
      count=$((count+1))
      echo ""
      echo "[$count/$total] Resolving latest version of core/$p"
      latest=$(curl -s -H "Accept: application/json" "$upstream_depot/v1/depot/channels/core/stable/pkgs/$p/latest")
      raw_ident=$(echo "$latest" | jq ".ident")

      if [ "$raw_ident" = "null" ]; then
        echo "Failed to find a latest version. Skipping."
        continue
      fi

      slash_ident=$(echo "$raw_ident" | jq "\"\(.origin)/\(.name)/\(.version)/\(.release)\"" | tr -d '"')
      dash_ident=$(echo "$raw_ident" | jq "\"\(.origin)-\(.name)-\(.version)-\(.release)\"" | tr -d '"')
      target=$(echo "$latest" | jq ".target" | tr -d '"')

      echo "[$count/$total] Downloading $slash_ident"
      curl -s -H "Accept: application/json" -o $tmp_dir/$dash_ident-$target.hart "$upstream_depot/v1/depot/pkgs/$slash_ident/download"
    done
    popd

    cd /tmp
    tar zcvf $tar_file -C $tmp_dir .
    s3_cp $tar_file s3://$bucket/
    s3_cp s3://$bucket/$bootstrap_file s3://$bucket/$marker

    if [ -z "$HAB_ON_PREM_BOOTSTRAP_DONT_CLEAN_UP" ]; then
      echo "Cleaning up."
      rm -fr "$tmp_dir"
      rm -fr "$core_tmp"
      rm "$tar_file"
    else
      echo "Cleanup skipped."
    fi

    echo "Done."
    ;;
  populate-depot)
    if [ -z "$2" ]; then
      help
      exit 1
    fi

    required_tools=( curl )
    required_vars=( HAB_AUTH_TOKEN )

    check_tools required_tools
    check_vars required_vars

    s3_root_url="${HAB_ON_PREM_BOOTSTRAP_S3_ROOT_URL:-https://s3-us-west-2.amazonaws.com}/$bucket"
    tmp_dir=$(mktemp -d)

    cd "$tmp_dir"
    echo "Fetching latest package bootstrap file."
    curl -O "$s3_root_url/$marker"
    tar zxvf $marker

    harts=$(find . -type f -name "*.hart")
    total=$(echo "$harts" | wc -l)
    count="0"

    for hart in $harts
    do
      count=$((count+1))
      echo ""
      echo "[$count/$total] Uploading $hart to the depot at $2"
      hab pkg upload --url $2 --channel stable $hart
    done

    echo "Package uploads finished."

    if [ -z "$HAB_ON_PREM_BOOTSTRAP_DONT_CLEAN_UP" ]; then
      echo "Cleaning up."
      cd /tmp
      rm -fr $tmp_dir
    else
      echo "Cleanup skipped."
    fi

    echo "Done."
    ;;
  *)
    help
    exit 1
esac
