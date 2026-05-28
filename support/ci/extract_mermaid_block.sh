#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "usage: $0 <markdown-file> <output-mmd-file>" >&2
    exit 1
fi

input_file=$1
output_file=$2

awk '
    /^```mermaid[[:space:]]*$/ { in_block = 1; next }
    /^```[[:space:]]*$/ && in_block { exit }
    in_block { print }
' "$input_file" > "$output_file"

if [[ ! -s "$output_file" ]]; then
    echo "no mermaid block found in $input_file" >&2
    exit 1
fi
