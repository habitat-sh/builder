pkg_origin=habitat
pkg_name=builder-datastore
pkg_internal_version=11.2
pkg_internal_name=postgresql11
pkg_description="Datastore service for a Habitat Builder service"
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=("PostgreSQL")
pkg_source="https://ftp.postgresql.org/pub/source/v${pkg_internal_version}/postgresql-${pkg_internal_version}.tar.bz2"
pkg_shasum="2676b9ce09c21978032070b6794696e0aa5a476e3d21d60afc036dc0a9c09405"
pkg_dirname="postgresql-${pkg_internal_version}"

pkg_deps=(
  core/bash
  core/glibc
  core/perl
  core/readline
  core/zlib
  core/libossp-uuid
)

pkg_build_deps=(
  core/coreutils
  core/gcc
  core/make
  core/git
)

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

ext_semver_version=0.17.0
ext_semver_source=https://github.com/theory/pg-semver/archive/v${ext_semver_version}.tar.gz
ext_semver_filename=pg-semver-${ext_semver_version}.tar.gz
ext_semver_shasum=031046695b143eb545a2856c5d139ebf61ae4e2f68cccb1f21b700ce65d0cd60

do_before() {
  update_pkg_version
  ext_semver_dirname="pg-semver-${ext_semver_version}"
  ext_semver_cache_path="$HAB_CACHE_SRC_PATH/${ext_semver_dirname}"
}

do_download() {
  do_default_download
  download_file $ext_semver_source $ext_semver_filename $ext_semver_shasum
}

do_verify() {
  do_default_verify
  verify_file $ext_semver_filename $ext_semver_shasum
}

do_clean() {
  do_default_clean
  rm -rf "$ext_semver_cache_path"
}

do_unpack() {
  do_default_unpack
  unpack_file $ext_semver_filename
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

  pushd "$ext_semver_cache_path" > /dev/null || exit
  build_line "Building ${ext_semver_dirname}"
  make
  build_line "Installing ${ext_semver_dirname}"
  make install
  popd > /dev/null || exit
}
