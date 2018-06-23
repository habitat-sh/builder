pkg_origin=habitat
pkg_name=builder-api-proxy
pkg_description="HTTP Proxy service fronting the Habitat Builder API service"
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_license=("Apache-2.0")
pkg_deps=(
  core/nginx/1.13.10/20180502174530
  core/curl/7.54.1/20180419094759
  core/coreutils/8.25/20170513213226
)
pkg_build_deps=(
  core/node8/8.6.0/20171102163558
  core/gcc/5.2.0/20170513202244
  core/git/2.14.2/20180416203520
  core/tar/1.29/20170513213607
  core/phantomjs/2.1.1/20180423190907
  core/python2/2.7.14/20180419094026
  core/make/4.2.1/20170513214620
)
pkg_svc_user="root"
pkg_svc_run="nginx -c ${pkg_svc_config_path}/nginx.conf"
pkg_exports=(
  [port]=server.listen_port
  [ssl-port]=server.listen_tls_port
  [url]=app_url
)
pkg_binds=(
  [http]="port"
)
pkg_exposes=(port ssl-port)

pkg_version() {
  # TED: After migrating the builder repo we needed to add to
  # the rev-count to keep version sorting working
  echo "$(($(git rev-list master --count) + 5000))"
}

do_before() {
  update_pkg_version
}

do_unpack() {
  pushd "$PLAN_CONTEXT/../../builder-web" > /dev/null
  { git ls-files; git ls-files --exclude-standard --others; } \
  | _tar_pipe_app_cp_to "${HAB_CACHE_SRC_PATH}"
  popd > /dev/null
}

do_build() {
  pushd $HAB_CACHE_SRC_PATH > /dev/null
  export HOME=$HAB_CACHE_SRC_PATH
  export PATH=./node_modules/.bin:$PATH
  npm install
  for b in node_modules/.bin/*; do
    fix_interpreter $(readlink -f -n $b) core/coreutils bin/env
  done

  # Pass the release identifier to the bundle script to enable cache-busting
  npm run dist -- ${pkg_prefix: -14}

  rm -rf dist/node_modules
  popd > /dev/null
}

do_install() {
  cp -a "${HAB_CACHE_SRC_PATH}/dist/." "${pkg_prefix}/app/"
}

_tar_pipe_app_cp_to() {
  local dst_path tar
  dst_path="$1"
  tar="$(pkg_path_for tar)/bin/tar"
  "$tar" -cp \
  --owner=root:0 \
  --group=root:0 \
  --no-xattrs \
  --exclude-backups \
  --exclude-vcs \
  --exclude='habitat' \
  --files-from=- \
  -f - \
  | "$tar" -x \
  -C "$dst_path" \
  -f -
}
