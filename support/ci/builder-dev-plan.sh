source ../../../support/ci/builder-dev-base-plan.sh

pkg_build_deps+=(core/sccache)
# shellcheck disable=2034
pkg_origin=habitat-dev

do_dev_prepare() {
  # Order matters here
  export CARGO_HOME="/tmp/cargo_cache"
  export builder_build_type="--debug"
  export RUSTC_WRAPPER
  RUSTC_WRAPPER="$(pkg_path_for core/sccache)/bin/sccache"
  export SCCACHE_DIR="/tmp/cargo_cache"
  export SCCACHE_START_SERVER=0
  do_builder_prepare
  export CARGO_TARGET_DIR="/tmp/target"
  PLAN_CONTEXT="../habitat"
  export RUST_BACKTRACE=1
}

do_prepare() {
  do_dev_prepare
}

# shellcheck disable=2154
do_builder_install() {
  local pkg_path
  pkg_path=$(hab pkg path habitat/"$pkg_name")

  build_line "Linking new binary into package"
  ln -sfv "$CARGO_TARGET_DIR/$rustc_target/${builder_build_type#--}/$bin" \
    "$pkg_path/bin/$bin"

  build_line "Copying new config into package"
  cp -v "$PLAN_CONTEXT/default.toml" "$pkg_path/default.toml"
  cp -v "$PLAN_CONTEXT/config/config.toml" "$pkg_path/config/config.toml"

  build_line "Copying run hooks into package"
  for hook in "$PLAN_CONTEXT"/hooks/*; do
    cp -v "$hook" "$pkg_path/hooks/$(basename "$hook")"
  done
}

do_install_wrapper() {
  do_install
}
