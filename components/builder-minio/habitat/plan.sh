# shellcheck shell=bash

pkg_name=builder-minio
pkg_origin=habitat
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=('Apache-2.0')
pkg_deps=(core/minio core/cacerts core/aws-cli core/bash)
pkg_build_deps=(core/git)

pkg_exports=(
    [port]=bind_port
    [bucket-name]=bucket_name
    [minio-access-key]=env.MINIO_ACCESS_KEY
    [minio-secret-key]=env.MINIO_SECRET_KEY
)

pkg_version() {
  # TED: After migrating the builder repo we needed to add to
  # the rev-count to keep version sorting working
  echo "$(($(git rev-list master --count) + 5000))"
}

do_before() {
  update_pkg_version
}

do_unpack() {
    return 0
}

do_build(){
    return 0
}

do_install() {
    return 0
}

do_strip() {
    return 0
}
