pkg_name=s3bulk
pkg_origin=habitat
pkg_version="0.1.0"
pkg_description="bulk s3 uploader"
pkg_upstream_url="https://github.com/habitat-sh/builder"
pkg_license=('Apache-2.0')
pkg_maintainer="The Habitat Maintainers <humans@habitat.sh>"
pkg_bin_dirs=(bin)
pkg_deps=(core/hab)
pkg_build_deps=(core/go core/git core/dep)
export GOPATH="${HAB_CACHE_SRC_PATH}/go"
export workspace_src="${GOPATH}/src"
export base_path="github.com/habitat-sh"
export pkg_cache_path="${workspace_src}/${base_path}/${pkg_name}"

do_before() {
  mkdir -p "$pkg_cache_path"
}

do_download() {
  cp -r "${PLAN_CONTEXT}/"* "$pkg_cache_path"
  pushd "${pkg_cache_path}" >/dev/null
  	dep ensure
    	go get
  popd >/dev/null
}

do_build() {
  pushd "${pkg_cache_path}" >/dev/null
  	GOOS=linux go build -o "${GOPATH}/bin/${pkg_name}" .
  popd >/dev/null
}

do_install() {
  cp -r "${GOPATH}/bin" "${pkg_prefix}/${bin}"
}
