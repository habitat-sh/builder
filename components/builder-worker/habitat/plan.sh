# shellcheck disable=SC2034
source "../../../support/ci/builder-base-plan.sh"
pkg_name=builder-worker
pkg_origin=habitat
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=('Apache-2.0')
pkg_bin_dirs=(bin)
# TODO: Remove pins after Habitat 1.6.23 or later is released
pkg_deps=(
    core/glibc/2.27/20190115002733
    core/openssl/1.0.2r/20190305210149
    core/gcc-libs/8.2.0/20190115011926
    core/zeromq/4.3.1/20190802173651
    core/libsodium/1.0.16/20190116014025
    core/libarchive/3.3.3/20190305214120
    core/zlib/1.2.11/20190115003728
    core/hab/1.6.0/20200420200029
    core/hab-studio/1.6.0/20200420202004
    core/hab-pkg-export-docker/1.6.0/20200420202330
    core/docker/19.03.3/20191025053746
    core/curl/7.68.0/20200309012427
)
pkg_build_deps=(
    core/make/4.2.1/20190115013626
    core/cmake/3.16.0/20191204143727
    core/protobuf-cpp/3.9.2/20191001233803
    core/protobuf-rust/1.7.4/20190116233225
    core/coreutils/8.30/20190115012313
    core/cacerts/2018.12.05/20190115014206
    core/rust/1.41.0/20200217103343
    core/gcc/8.2.0/20190115004042
    core/git/2.25.1/20200309023931
    core/pkg-config/0.29.2/20190115011955
)
pkg_binds=(
  [jobsrv]="worker_port worker_heartbeat log_port"
  [depot]="url"
)
pkg_svc_user="root"
pkg_svc_group="root"
bin="bldr-worker"

do_prepare() {
  do_builder_prepare

  # Used by libssh2-sys
  export DEP_Z_ROOT DEP_Z_INCLUDE
  DEP_Z_ROOT="$(pkg_path_for zlib)"
  DEP_Z_INCLUDE="$(pkg_path_for zlib)/include"

  # Compile the fully-qualified hab cli package identifier into the binary
  PLAN_HAB_PKG_IDENT=$(pkg_path_for hab | sed "s,^$HAB_PKG_PATH/,,")
  export PLAN_HAB_PKG_IDENT
  build_line "Setting PLAN_HAB_PKG_IDENT=$PLAN_HAB_PKG_IDENT"

  # Compile the fully-qualified Studio package identifier into the binary
  PLAN_STUDIO_PKG_IDENT=$(pkg_path_for hab-studio | sed "s,^$HAB_PKG_PATH/,,")
  export PLAN_STUDIO_PKG_IDENT
  build_line "Setting PLAN_STUDIO_PKG_IDENT=$PLAN_STUDIO_PKG_IDENT"

  # Compile the fully-qualified Docker exporter package identifier into the binary
  PLAN_DOCKER_EXPORTER_PKG_IDENT=$(pkg_path_for hab-pkg-export-docker | sed "s,^$HAB_PKG_PATH/,,")
  export PLAN_DOCKER_EXPORTER_PKG_IDENT
  build_line "Setting PLAN_DOCKER_EXPORTER_PKG_IDENT=$PLAN_DOCKER_EXPORTER_PKG_IDENT"

  # Compile the fully-qualified Docker package identifier into the binary
  PLAN_DOCKER_PKG_IDENT=$(pkg_path_for docker | sed "s,^$HAB_PKG_PATH/,,")
  export PLAN_DOCKER_PKG_IDENT
  build_line "Setting PLAN_DOCKER_PKG_IDENT=$PLAN_DOCKER_PKG_IDENT"
}
