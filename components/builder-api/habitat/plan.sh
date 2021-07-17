source "../../../support/ci/builder-base-plan.sh"
pkg_name=builder-api
pkg_origin=habitat
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=('Apache-2.0')
pkg_bin_dirs=(bin)
pkg_deps=(core/glibc core/openssl core/coreutils core/gcc-libs core/zeromq
core/libarchive core/curl core/postgresql)
pkg_build_deps=(core/protobuf-cpp core/protobuf-rust core/coreutils core/cacerts
core/rust core/gcc core/git core/pkg-config core/bash core/make core/cmake)
pkg_exports=(
  [port]=http.port
)
pkg_exposes=(port)
pkg_binds=(
  [memcached]="port"
)
pkg_binds_optional=(
  [jobsrv]="rpc_port"
)
bin="bldr-api"
