pkg_name=builder-minio
pkg_version="0.1.0"
pkg_origin=habitat
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=('Apache-2.0')
pkg_deps=(
  core/minio/2018-05-11T00-29-24Z/20180515134704
  core/cacerts/2017.09.20/20171014212239
  core/openssl/1.0.2l/20180419014054
)

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
