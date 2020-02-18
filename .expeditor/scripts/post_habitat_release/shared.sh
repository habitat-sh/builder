#!/bin/bash

# Download public and private keys for the given origin from Builder.
import_keys() {
    local origin="${1}"

    echo "--- :key: Downloading '${origin}' public keys from ${HAB_BLDR_URL}"
    hab origin key download "${origin}"
    echo "--- :closed_lock_with_key: Downloading latest '${origin}' secret key from ${HAB_BLDR_URL}"
    hab origin key download \
        --auth="${HAB_AUTH_TOKEN}" \
        --secret \
        "${origin}"
}

curlbash_hab() {
    local pkg_target="${1:-$BUILD_PKG_TARGET}"
    echo "--- :habicat: Installing current stable hab binary for $pkg_target using curl|bash"
    curl https://raw.githubusercontent.com/habitat-sh/habitat/master/components/hab/install.sh | sudo bash -s -- -t "$pkg_target"
}

set_worker_ident_for_target() {
    package_ident="${1}"
    target="${2}"

    echo "--- Registering ${package_ident} (${target})"
    buildkite-agent meta-data set "${target}-builder-worker" "${package_ident}"
}

worker_ident_for_target() {
    target="${1}"
    buildkite-agent meta-data get "${target}-builder-worker"
}

install_rustup() {
  if command -v rustup && command -v cargo &>/dev/null; then
    echo "--- :rust: rustup is currently installed."
  else
    echo "--- :rust: Installing rustup."
    curl https://sh.rustup.rs -sSf | sh -s -- --no-modify-path -y --profile=minimal
    # shellcheck disable=SC1090
    source "$HOME"/.cargo/env
  fi
}

get_toolchain() {
    cat "$(git rev-parse --show-toplevel)/rust-toolchain"
}

latest_release_tag() {
  local repo="${1?repo argument required}"
  tag=$(curl --silent "https://api.github.com/repos/${repo}/releases/latest" | jq -r .tag_name)
  echo "${tag}"
}

install_hub() {
  # TODO: Create a Hab core plans pkg for this.
  # see https://github.com/habitat-sh/habitat/issues/7267
  local tag
  tag=$(latest_release_tag github/hub)
  tag_sans_v="${tag//v/}"
  url="https://github.com/github/hub/releases/download/${tag}/hub-linux-amd64-${tag_sans_v}.tgz"
  echo "--- :github: Installing hub version ${tag} to /bin/hub from ${url}"
  curl -L -O "${url}"
  tar xfz hub-linux-amd64-*.tgz
  cp -f hub-linux-amd64-*/bin/hub /bin
  chmod a+x /bin/hub
  rm -rf hub-linux-amd64*
}

# Push the current branch to the project origin
push_current_branch() {
  repo=$(git remote get-url origin | sed -rn  's/.+github\.com[\/\:](.*)\.git/\1/p')
  head=$(git rev-parse --abbrev-ref HEAD)

  if [ "$head" == "master" ]; then
    echo "Error: Attempting to push to master!"
    exit 1
  fi

  git push "https://x-access-token:${GITHUB_TOKEN}@github.com/${repo}.git" "$head"
}
