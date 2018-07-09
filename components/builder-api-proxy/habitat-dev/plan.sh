# shellcheck disable=SC2046,SC2086,SC2154
source ../habitat/plan.sh
source ../../../support/ci/builder-dev-base-plan.sh

pkg_origin=habitat-dev

do_build() {
  pushd $HAB_CACHE_SRC_PATH > /dev/null
  export HOME=$HAB_CACHE_SRC_PATH
  export PATH=./node_modules/.bin:$PATH
  npm install

  for b in node_modules/.bin/*; do
    fix_interpreter $(readlink -f -n $b) core/coreutils bin/env
  done

  # NPM install creates an "etc' folder in the pkg_prefix dir
  # because we have a package that uses the PREFIX env var during install
  # We don't want pkg_prefix to have content, so delete the directory now
  rm -rf ${pkg_prefix}

  # Pass the release identifier to the bundle script to enable cache-busting
  # Create the dist with the currently installed package version number as we
  # are going to overwrite it with the new app and js
  local pkg_path
  pkg_path=$(hab pkg path habitat/"$pkg_name")
  build_line "Creating the NPM dist with cache buster: ${pkg_path: -14}"
  npm run dist -- ${pkg_path: -14}

  rm -rf dist/node_modules
  popd > /dev/null
}

do_install() {
  # We don't want pkg_prefix to have content, so delete the directory before
  # install
  rm -rf ${pkg_prefix}

  local pkg_path
  pkg_path=$(hab pkg path habitat/"$pkg_name")

  build_line "Copying app into existing path ${pkg_path}/app"
  cp -a "${HAB_CACHE_SRC_PATH}/dist/." "${pkg_path}/app/"
}
