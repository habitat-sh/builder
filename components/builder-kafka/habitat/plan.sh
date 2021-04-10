pkg_name=builder-kafka
pkg_origin=habitat
pkg_version=2.7.0
pkg_dirname="kafka_2.13-${pkg_version}"
pkg_filename="${pkg_dirname}.tgz"
pkg_source="https://downloads.apache.org/kafka/${pkg_version}/${pkg_filename}"
pkg_shasum="1dd84b763676a02fecb48fa5d7e7e94a2bf2be9ff87bce14cf14109ce1cb7f90"
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_description="A distributed streaming platform"
pkg_upstream_url="https://kafka.apache.org/"
pkg_license=('Apache-2.0')
pkg_bin_dirs=(bin)
pkg_svc_user="root"
pkg_svc_group="root"
pkg_deps=(
  core/bash-static
  core/coreutils
  core/corretto11
)

do_build() {
  fix_interpreter "./bin/*" core/bash-static bin/bash
}

do_install() {
  cp -R libs bin "${pkg_prefix}"
}
