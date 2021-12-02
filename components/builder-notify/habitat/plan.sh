source "../../../support/ci/builder-base-plan.sh"
pkg_name=builder-notify
pkg_origin=habitat
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=('Apache-2.0')
pkg_bin_dirs=(bin)
pkg_deps=(core/glibc
          core/openssl
          core/coreutils
          core/gcc-libs
          core/zeromq
          core/libarchive
          core/curl
          core/postgresql)
pkg_build_deps=(core/protobuf-cpp
                core/protobuf-rust
                core/coreutils
                core/cacerts
                core/cmake
                core/rust/"$(cat "../../../rust-toolchain")"
                core/gcc
                core/git
                core/pkg-config
                core/bash
                core/make)
pkg_exports=()
pkg_exposes=()
pkg_binds=()
pkg_binds_optional=()
bin="bldr-notify"
