#!/usr/bin/env bash

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
input_file=${1:-"$repo_root/docs/architecture.md"}
output_file=${2:-"${TMPDIR:-/tmp}/builder-architecture.svg"}
tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/builder-architecture.XXXXXX")

cleanup() {
    rm -rf "$tmp_dir"
}

trap cleanup EXIT

"$repo_root/support/ci/extract_mermaid_block.sh" "$input_file" "$tmp_dir/architecture.mmd"

npx -y @mermaid-js/mermaid-cli@11 \
    -p "$repo_root/support/ci/mermaid-puppeteer-config.json" \
    -i "$tmp_dir/architecture.mmd" \
    -o "$output_file"

if [[ ! -s "$output_file" ]]; then
    echo "failed to render architecture diagram to $output_file" >&2
    exit 1
fi

echo "rendered architecture diagram to $output_file"
