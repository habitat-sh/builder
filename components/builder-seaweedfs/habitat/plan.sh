#!/bin/bash
# shellcheck disable=SC2034

pkg_name=builder-seaweedfs
pkg_origin=habitat
pkg_description="SeaweedFS S3-compatible artifact storage for Habitat Builder"
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=('Apache-2.0')
pkg_deps=(core/aws-cli core/bash core/coreutils core/jq-static core/seaweedfs/4.55)
pkg_build_deps=(core/git)
pkg_svc_user=hab
pkg_svc_group=hab
pkg_exports=(
  [port]=bind_port
  [bucket-name]=bucket_name
)
pkg_exposes=(port)

pkg_version() {
  echo "$(($(git rev-list HEAD --count) + 5000))"
}

do_before() {
  git config --global --add safe.directory /src
  update_pkg_version
}

do_unpack() { return 0; }
do_build() { return 0; }
do_install() { return 0; }
do_strip() { return 0; }
