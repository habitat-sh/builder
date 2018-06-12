
#!/bin/bash

# Given an "environment", generates a tmuxinator YAML on STDOUT to
# connect to all the Habitat Builder hosts in that environment.

set -euo pipefail

environment=${1}

# Note: if no hosts are found (and thus no windows generated),
# tmuxinator will refuse to start, which is nice.

echo "# Auto-generated by hab-env. All manual edits will be lost!"
echo "name: hab_${environment}"
echo "windows:"
# shellcheck disable=2013
for host in $(grep "Host ${environment}-builder" ~/.ssh/config | awk '{print $2}' | sort)
do
    title=${host#${environment}-builder-}
    echo "  - ${title}:"
    echo "    - ssh $host"
done
