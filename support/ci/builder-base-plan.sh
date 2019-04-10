pkg_version() {
  # TED: After migrating the builder repo we needed to add to
  # the rev-count to keep version sorting working
  echo "$(($(git rev-list master --count) + 5000))"
}

do_before() {
  do_builder_before
}

do_prepare() {
  do_builder_prepare
}

do_build() {
  do_builder_build
}

do_install() {
  do_builder_install
}

do_strip() {
  return 0
}

do_builder_before() {
  update_pkg_version
}

do_builder_build() {
  pushd "$PLAN_CONTEXT"/.. > /dev/null || exit
  cargo build "${builder_build_type#--debug}" --target="$rustc_target" --verbose
  popd > /dev/null || exit
}

do_builder_install() {
  # shellcheck disable=2154
  install -v -D "$CARGO_TARGET_DIR/$rustc_target/${builder_build_type#--}/$bin" \
    "$pkg_prefix/bin/$bin"
}

# shellcheck disable=2154
do_builder_prepare() {
  export builder_build_type="${builder_build_type:---release}"
  # Can be either `--release` or `--debug` to determine cargo build strategy
  build_line "Building artifacts with \`${builder_build_type#--}' mode"

  export rustc_target="x86_64-unknown-linux-gnu"
  build_line "Setting rustc_target=$rustc_target"

  # Used by the `build.rs` program to set the version of the binaries
  export PLAN_VERSION="${pkg_version}/${pkg_release}"
  build_line "Setting PLAN_VERSION=$PLAN_VERSION"

  # Used by Cargo to use a pristine, isolated directory for all compilation
  export CARGO_TARGET_DIR="$HAB_CACHE_SRC_PATH/$pkg_dirname"
  build_line "Setting CARGO_TARGET_DIR=$CARGO_TARGET_DIR"

  # Used to set the active package target for the binaries at build time
  export PLAN_PACKAGE_TARGET="$pkg_target"
  build_line "Setting PLAN_PACKAGE_TARGET=$PLAN_PACKAGE_TARGET"
}
