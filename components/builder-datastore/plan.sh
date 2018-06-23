pkg_origin=habitat
pkg_name=builder-datastore
pkg_description="Datastore service for a Habitat Builder service"
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=("Apache-2.0")
pkg_deps=(
  core/postgresql/9.6.8/20180426174635
)
pkg_build_deps=(
  core/git/2.14.2/20180416203520
)
pkg_exports=(
  [port]=port
)
pkg_exposes=(port)

pkg_version() {
  # TED: After migrating the builder repo we needed to add to
  # the rev-count to keep version sorting working
  echo "$(($(git rev-list master --count) + 5000))"
}

do_before() {
  update_pkg_version
}

do_build() {
  return 0
}

do_install() {
  return 0
}
