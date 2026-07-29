#!/usr/bin/env bash
# generate-habitat-pkg-deps.sh
#
# Queries the public Builder API for the full transitive dependency trees
# (tdeps) of this repo's top-level, deployable Habitat packages and emits a
# CycloneDX fragment covering their core-origin runtime dependencies.
#
# Motivation: only components/builder-api is a Rust project (its SBOM is
# generated separately via cargo-cyclonedx). The other deployable components
# (builder-api-proxy, builder-memcached, builder-minio, builder-datastore)
# wrap third-party binaries packaged as Habitat "core" packages, which are
# invisible to cargo tooling. This script captures those dependencies by
# asking Builder directly for each top-level package's dependency tree, the
# same way /habitat's support/sbom scripts derive its core-origin fragment.
#
# All top-level packages here are built for x86_64-linux only, so unlike
# /habitat's multi-platform SBOM job, only a single target is queried.
#
# Usage:
#   bash support/sbom/generate-habitat-pkg-deps.sh > habitat-pkg-deps.cdx.json
#
# Environment:
#   BLDR_URL   Builder base URL      (default: https://bldr.habitat.sh)
#   CHANNEL    Channel to inspect    (default: on-prem-base)
#   TARGET     Habitat package target (default: x86_64-linux)
#
# Requires: curl, jq

set -euo pipefail

BLDR_URL="${BLDR_URL:-https://bldr.habitat.sh}"
CHANNEL="${CHANNEL:-on-prem-base}"
TARGET="${TARGET:-x86_64-linux}"

# The top-level, deployable Habitat packages that make up this product.
TOP_LEVEL_PACKAGES=(
  "habitat/builder-api-proxy"
  "habitat/builder-api"
  "habitat/builder-memcached"
  "habitat/builder-minio"
  "habitat/builder-datastore"
)

echo "Builder URL:     $BLDR_URL" >&2
echo "Channel:         $CHANNEL" >&2
echo "Target:          $TARGET" >&2
echo "Top-level pkgs:  ${TOP_LEVEL_PACKAGES[*]}" >&2
echo "" >&2

# CORE_DEPS is a set keyed by "name@version" to deduplicate across all
# top-level packages' dependency trees.
declare -A CORE_DEPS       # key: "name@version" -> "1"
declare -A CORE_DEP_META   # key: "name@version" -> "name version"

add_dep() {
  local name="$1" version="$2"
  local key="${name}@${version}"
  if [ -z "${CORE_DEPS[$key]+_}" ]; then
    CORE_DEPS["$key"]="1"
    CORE_DEP_META["$key"]="${name} ${version}"
  fi
}

for pkg in "${TOP_LEVEL_PACKAGES[@]}"; do
  origin="${pkg%%/*}"
  pkg_name="${pkg##*/}"
  url="${BLDR_URL}/v1/depot/channels/${origin}/${CHANNEL}/pkgs/${pkg_name}/latest?target=${TARGET}"
  printf "  Fetching %-40s [%-16s] ... " "${pkg}" "${TARGET}" >&2
  if response=$(curl -sSf "$url" 2>/dev/null); then
    mapfile -t deps < <(
      echo "$response" \
        | jq -r '.tdeps[]? | select(.origin == "core") | "\(.name)/\(.version)"' \
        2>/dev/null \
      || true
    )
    printf "%d core deps\n" "${#deps[@]}" >&2
    for dep in "${deps[@]}"; do
      [ -z "$dep" ] && continue
      add_dep "${dep%%/*}" "${dep##*/}"
    done
  else
    echo "ERROR: failed to fetch ${pkg} from channel ${CHANNEL} (target ${TARGET})" >&2
    exit 1
  fi
done

echo "" >&2
echo "Unique core {name, version} pairs found: ${#CORE_DEPS[@]}" >&2
echo "" >&2

if [ "${#CORE_DEPS[@]}" -eq 0 ]; then
  echo "ERROR: No core-origin packages found. Check BLDR_URL, CHANNEL, and TOP_LEVEL_PACKAGES." >&2
  exit 1
fi

# Build one CycloneDX component per unique {name, version}, using the same
# naming/purl convention as /habitat's habitat-core-deps.cyclonedx.json so
# BlackDuck maps them to the same KB entries rather than creating duplicates.
component_jsons=()
for key in $(printf '%s\n' "${!CORE_DEP_META[@]}" | sort); do
  read -r pkg_name version <<< "${CORE_DEP_META[$key]}"
  display_name="Habitat core_${pkg_name}"
  purl="pkg:generic/${pkg_name}@${version}"
  component_jsons+=(
    "$(jq -cn \
      --arg name    "$display_name" \
      --arg version "$version" \
      --arg purl    "$purl" \
      '{type: "library", name: $name, version: $version, purl: $purl}')"
  )
done

components_json=$(printf '%s\n' "${component_jsons[@]}" | jq -s '.')

jq -n \
  --argjson components "$components_json" \
  '{
    bomFormat: "CycloneDX",
    specVersion: "1.4",
    version: 1,
    components: $components
  }'
