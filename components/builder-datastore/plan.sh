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
  # shellcheck disable=2154
  # ld manpage: "If -rpath is not used when linking an ELF
  # executable, the contents of the environment variable LD_RUN_PATH
  # will be used if it is defined"
  ./configure --disable-rpath \
    --with-openssl \
    --prefix="$pkg_prefix" \
    --with-uuid=ossp \
    --with-includes="$LD_INCLUDE_PATH" \
    --with-libraries="$LD_LIBRARY_PATH" \
    --sysconfdir="$pkg_svc_config_path" \
    --localstatedir="$pkg_svc_var_path"
  make world

  # semver can't be built until after postgresql is installed to $pkg_prefix
}

do_install() {
  make install-world

  # make and install semver extension
  export PATH="${PATH}:${pkg_prefix}/bin"
  build_line "Added postgresql binaries to PATH: ${pkg_prefix}/bin"

  pushd "$ext_semver_cache_path" >/dev/null || exit
  build_line "Building ${ext_semver_dirname}"
  make
  build_line "Installing ${ext_semver_dirname}"
  make install
  popd >/dev/null || exit
}
