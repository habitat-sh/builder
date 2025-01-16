#!/bin/bash
#shellcheck disable=SC2034

pkg_origin=habitat
pkg_name=builder-datastore
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=('Apache-2.0')

pkg_description="Datastore service for a Habitat Builder service"

pkg_deps=(core/postgresql17)

pkg_build_deps=(core/git)

pkg_bin_dirs=(bin)
pkg_include_dirs=(include)
pkg_lib_dirs=(lib)
pkg_exports=(
  [port]=port
)
pkg_exposes=(port)

pkg_version() {
  # TED: After migrating the builder repo we needed to add to
  # the rev-count to keep version sorting working
  echo "$(($(git rev-list HEAD --count) + 5000))"
}

do_before() {
  git config --global --add safe.directory /src
  update_pkg_version
}

do_build() {
  return 0
}

do_install() {
  return 0
}
