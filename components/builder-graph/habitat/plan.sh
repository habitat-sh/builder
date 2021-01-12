# shellcheck disable=SC2034
source "../../../support/ci/builder-base-plan.sh"
pkg_name=builder-graph
pkg_origin=habitat
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=('Apache-2.0')
pkg_bin_dirs=(bin)
pkg_deps=(
  core/glibc
  core/openssl
  core/gcc-libs
  core/libarchive
  core/postgresql
  core/zeromq #TODO: This can probably be removed if we removed the crate dep on builder-protocol
  core/zlib
  core/xz
)
pkg_build_deps=(
  core/protobuf-cpp #TODO: This can probably be removed if we removed the crate dep on builder-protocol
  core/protobuf-rust #TODO: This can probably be removed if we removed the crate dep on builder-protocol
  core/rust
  core/pkg-config
  core/git
)
bin="bldr-graph"

