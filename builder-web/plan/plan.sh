pkg_name=habitat-builder-web
pkg_version=0.4.0
pkg_origin=core
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=('apachev2')
pkg_source=nosuchfile.tar.gz
pkg_filename=${pkg_name}-${pkg_version}.tar.bz2
pkg_deps=(core/coreutils core/nginx)
pkg_build_deps=(core/coreutils core/node core/phantomjs)
pkg_expose=(80)

do_prepare() {
  rm -Rdf $HAB_CACHE_SRC_PATH/$pkg_name-$pkg_version
  cp -ra $PLAN_CONTEXT/.. $HAB_CACHE_SRC_PATH/$pkg_name-$pkg_version
}

do_build() {
  export HOME=$HAB_CACHE_SRC_PATH

  npm install

  for b in $HAB_CACHE_SRC_PATH/$pkg_name-$pkg_version/node_modules/.bin/*; do
    echo $b
    fix_interpreter $(readlink -f -n $b) core/coreutils bin/env
  done

  npm run postinstall
  npm run dist
}

do_install() {
  cp -vR dist ${pkg_prefix}/dist
}

do_download() {
  return 0
}

do_verify() {
  return 0
}

do_unpack() {
  return 0
}
