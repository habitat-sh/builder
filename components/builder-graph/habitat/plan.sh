source "../../../support/ci/builder-base-plan.sh"
pkg_name=builder-graph
pkg_origin=habitat
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=('Apache-2.0')
pkg_bin_dirs=(bin)
pkg_deps=(
  core/glibc/2.22/20170513201042
  core/openssl/1.0.2l/20180419014054
  core/gcc-libs/5.2.0/20170513212920
  core/zeromq/4.2.5/20180407102804
  core/libsodium/1.0.13/20170905223149
  core/libarchive/3.3.2/20171018164107
  core/zlib/1.2.8/20170513201911
  core/hab-studio
  core/curl/7.54.1/20180419094759
  core/postgresql/9.6.8/20180426174635
)
pkg_build_deps=(
  core/make/4.2.1/20170513214620
  core/cmake/3.10.2/20180418232649
  core/protobuf-cpp/3.5.0/20180418230816
  core/protobuf-rust/1.4.4/20180418221745
  core/coreutils/8.25/20170513213226
  core/cacerts/2017.09.20/20171014212239
  core/rust/1.26.2/20180606182054
  core/gcc/5.2.0/20170513202244
  core/git/2.14.2/20180416203520
  core/pkg-config/0.29/20170513212944
)
bin="bldr-graph"

do_prepare() {
  do_builder_prepare

  # Used by libssh2-sys
  export DEP_Z_ROOT="$(pkg_path_for zlib)"
  export DEP_Z_INCLUDE="$(pkg_path_for zlib)/include"
}
