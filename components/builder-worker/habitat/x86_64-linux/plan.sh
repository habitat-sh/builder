# shellcheck disable=SC2034
source "../../../../support/ci/builder-base-plan.sh"
pkg_name=builder-worker
pkg_origin=habitat
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=('Apache-2.0')
pkg_bin_dirs=(bin)
pkg_deps=(core/glibc core/gcc-libs core/zeromq
  core/libarchive core/zlib core/hab core/hab-studio core/hab-pkg-export-container
  core/docker core/curl)
pkg_build_deps=(core/make core/cmake core/protobuf-cpp core/protobuf-rust core/coreutils core/cacerts
  core/rust core/gcc core/git core/pkg-config)
pkg_binds=(
  [jobsrv]="worker_port worker_heartbeat log_port"
  [depot]="url"
)
pkg_svc_user="root"
pkg_svc_group="root"
bin="bldr-worker"

# Copy hooks/config/default.toml from parent directory so we only maintain
# one copy.
do_begin() {
  mkdir -p hooks
  mkdir -p config
  cp --no-clobber ../_common/run hooks/run
  cp --no-clobber ../_common/config.toml config/config.toml
  cp --no-clobber ../_common/default.toml default.toml
}

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
  PLAN_CONTAINER_EXPORTER_PKG_IDENT=$(pkg_path_for hab-pkg-export-container | sed "s,^$HAB_PKG_PATH/,,")
  export PLAN_CONTAINER_EXPORTER_PKG_IDENT
  build_line "Setting PLAN_CONTAINER_EXPORTER_PKG_IDENT=$PLAN_CONTAINER_EXPORTER_PKG_IDENT"

  # Compile the fully-qualified Docker package identifier into the binary
  PLAN_DOCKER_PKG_IDENT=$(pkg_path_for docker | sed "s,^$HAB_PKG_PATH/,,")
  export PLAN_DOCKER_PKG_IDENT
  build_line "Setting PLAN_DOCKER_PKG_IDENT=$PLAN_DOCKER_PKG_IDENT"
}
