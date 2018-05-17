# For dev purposes only - this bypasses artifact creation and other important
# parts of the full plan build process

do_clean() {
  build_line "Leaving $CACHE_PATH entact"
  return 0
}

do_build_config() {
  return 0
}

do_build_service() {
  return 0
}

_generate_artifact() {
  return 0
}

_render_metadata_FILES() {
  return 0
}

_build_manifest() {
  return 0
}

_prepare_build_outputs() {
  return 0
}

_build_metadata() {
  return 0
}

do_end() {
  # shellcheck disable=2154
  rm -rf "${pkg_prefix}/../../${pkg_version}"
}
