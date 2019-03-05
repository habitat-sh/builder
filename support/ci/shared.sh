#!/bin/bash

set -euo pipefail

get_current_toolchain() {
  # It turns out that every nightly version of rustfmt has slight tweaks from the previous version.
  # This means that if we're always using the latest version, then we're going to have enormous
  # churn. Even PRs that don't touch rust code will likely fail CI, since master will have been
  # formatted with a different version than is running in CI. Because of this, we're going to pin
  # the version of nightly that's used to run rustfmt and bump it when we do a new release.
  #
  # Note that not every nightly version of rust includes rustfmt. Sometimes changes are made that
  # break the way rustfmt uses rustc. Therefore, before updating the pin below, double check
  # that the nightly version you're going to update it to includes rustfmt. You can do that
  # using https://mexus.github.io/rustup-components-history/x86_64-unknown-linux-gnu.html
  echo "nightly-2019-03-04"
}

install_rust_toolchain() {
  local toolchain="${1?toolchain argument required}"

  if rustup component list --toolchain "$toolchain" &>/dev/null; then
    echo "--- :rust: Rust $toolchain is already installed."
  else
    echo "--- :rust: Installing rust $toolchain."
    rustup toolchain install "$toolchain"
  fi
}

install_rustfmt() {
  local toolchain="${1?toolchain argument required}"
  install_rust_toolchain "$toolchain"
  rustup component add --toolchain "$toolchain" rustfmt
}
