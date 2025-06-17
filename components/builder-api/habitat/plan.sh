#!/bin/bash
#shellcheck disable=SC2034

source "../../../support/ci/builder-base-plan.sh"

pkg_name=builder-api
pkg_origin=habitat
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=('Apache-2.0')
pkg_bin_dirs=(bin)
pkg_include_dirs=(include)
pkg_lib_dirs=(lib lib64)
pkg_pconfig_dirs=(lib/pkgconfig lib64/pkgconfig)

pkg_deps=(
  core/coreutils
  core/curl
  core/gcc-base
  core/glibc
  core/libarchive
  core/openssl
  core/postgresql17-client
  core/zeromq
)

pkg_build_deps=(
  core/cacerts
  core/cmake
  core/coreutils
  core/gcc
  core/git
  core/pkg-config
  core/protobuf-cpp
  core/protobuf-rust
  # core/rust/"$(tail -n 1 "../../../rust-toolchain" | cut -d'"' -f 2)"
  core/rust/1.79.0/20250606210134
)

pkg_exports=(
  [port]=http.port
)

pkg_exposes=(port)

pkg_binds=(
  [memcached]="port"
)

bin="bldr-api"
