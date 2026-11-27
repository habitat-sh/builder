#!/bin/bash
# Generated in CI rather than checking a large binary fixture into git.
pkg_name=s3-large
pkg_origin=s3test
pkg_version=0.1.0
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=('Apache-2.0')
pkg_description="Signed multipart upload fixture for Builder storage tests"

do_build() { return 0; }
do_install() {
  # Incompressible bytes ensure the resulting signed HART exceeds 20 MiB.
  dd if=/dev/urandom of="$pkg_prefix/payload" bs=1048576 count=21
}
