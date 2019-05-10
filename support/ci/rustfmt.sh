#!/bin/bash

set -euo pipefail

# This is problematic if you want to be able to run this script from anywhere other than the root of the project,
# but changing it to an idiom like we have in rustfmt.sh breaks BK, so I dunno?
# shellcheck disable=SC1094
source ./support/ci/shared.sh

echo "--- :rust: install rust"
sudo hab pkg install core/rust --binlink

echo "--- :rust: install rustup"
install_rustup

dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
# shellcheck disable=SC1090
source "$dir/shared.sh"
toolchain=$(get_current_toolchain)

install_rustup
install_rustfmt "$toolchain"

# A hack to avoid rustfmt choking on nonexistent generated files
# These can't be ignored because the error is rustc-level, before rustfmt
# Alternatively, we could do a `cargo build` first, but that'd take awhile.
# Because rustfmt requires an empty file to still have a newline, we can't
# just use `touch` here.
for generated_file in components/builder-protocol/src/message/{jobsrv,net,originsrv}.rs; do
	if [[ ! -s "$generated_file" ]]; then
		echo > "$generated_file"
	fi
done

touch components/builder-protocol/src/message/{jobsrv,net,originsrv}.rs
cargo_fmt="cargo +$toolchain fmt --all -- --check"
echo "--- :rust: Running cargo fmt command: $cargo_fmt"
$cargo_fmt
