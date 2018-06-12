#!/bin/bash

set -euo pipefail

shellcheck --version

# Run shellcheck against any files that appear to be shell script based on
# filename or `file` output
#
# Exclude *.sample files because they are automatically created by git
#
# Exclude *.ps1 files because shellcheck doesn't support them
#
# Exclude the following shellcheck issues since they're pervasive and innocuous:
# https://github.com/koalaman/shellcheck/wiki/SC1090
# https://github.com/koalaman/shellcheck/wiki/SC1091
# https://github.com/koalaman/shellcheck/wiki/SC2148
# https://github.com/koalaman/shellcheck/wiki/SC2034
find . -type f \
  -and \( -name "*.*sh" \
      -or -exec sh -c 'file -b "$1" | grep -q "shell script"' {} \; \) \
  -and \! -path "*.sample" \
  -and \! -path "*.ps1" \
  -print \
  | xargs shellcheck --external-sources --exclude=1090,1091,2148,2034

echo "shellcheck found no errors"
