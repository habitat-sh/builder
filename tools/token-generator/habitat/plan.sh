#!/bin/bash
#shellcheck disable=SC2034

source "../../../support/ci/builder-base-plan.sh"

pkg_name=builder-token-generator
pkg_origin=habitat
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=('Apache-2.0')
pkg_bin_dirs=(bin)

pkg_deps=(
  core/openssl
)

pkg_build_deps=(
  core/git
  core/pkg-config
  core/protobuf-cpp
  core/protobuf-rust
  core/rust/"$(tail -n 1 "../../../rust-toolchain" | cut -d'"' -f 2)"
)

bin="token-generator"
