#!/bin/bash

echo "--- CHECKING ldd --version at script start before shared_build_environment.sh called"
if command -v ldd >/dev/null 2>&1; then
  ldd --version
else
  echo "ldd not found?"
fi
echo ""

export RUSTFLAGS="-D warnings"

# Because sadness
if ${BUILDKITE:-false}; then
  sudo chown buildkite-agent /home/buildkite-agent
fi

echo "--- loading shared build environment "
# shellcheck source=../support/ci/shared_build_environment.sh
source support/ci/shared_build_environment.sh

toolchain="${1:-"$(get_toolchain)"}"

# Lints we need to work through and decide as a team whether to allow or fix
mapfile -t unexamined_lints <"$2"

# Lints we disagree with and choose to keep in our code with no warning
mapfile -t allowed_lints <"$3"

# Known failing lints we want to receive warnings for, but not fail the build
mapfile -t lints_to_fix <"$4"

# Lints we don't expect to have in our code at all and want to avoid adding
# even at the cost of failing the build
mapfile -t denied_lints <"$5"

clippy_args=()

add_lints_to_clippy_args() {
  flag=$1
  shift
  for lint; do
    clippy_args+=("$flag" "${lint}")
  done
}

set +u # See https://stackoverflow.com/questions/7577052/bash-empty-array-expansion-with-set-u/39687362#39687362
add_lints_to_clippy_args -A "${unexamined_lints[@]}"
add_lints_to_clippy_args -A "${allowed_lints[@]}"
add_lints_to_clippy_args -W "${lints_to_fix[@]}"
add_lints_to_clippy_args -D "${denied_lints[@]}"
set -u

echo "--- cat /etc/os-release"
if [[ -f /etc/os-release ]]; then
  cat /etc/os-release
else
  echo "/etc/os-release doesn't exist"
fi
echo ""

echo "--- cat /etc/system-release"
if [[ -f /etc/system-release ]]; then
  cat /etc/system-release
else
  echo "/etc/system-release doesn't exist"
fi
echo ""

echo "--- docker --version"
if command -v docker >/dev/null 2>&1; then
  docker --version
else
  echo "docker command unavailable"
fi
echo ""

echo "--- [[ -f /.dockerenv ]]"
if [[ -f /.dockerenv ]]; then
  echo "Running inside Docker, catting /.dockerenv"
  cat /.dockerenv
  echo "after the catting .dockerenv, if no line before this and after 'Running inside' file was empty"
else
  echo "[[-f /.dockerenv]] failed "
fi
echo ""

echo "--- grepping /proc/1/cgroup "
if grep -qE '(docker|kubepods)' /proc/1/cgroup 2>&1/dev/null; then
  echo "Running inside Docker based on grepping /proc/1/cgroup, running the command again"
  grep -qE '(docker|kubepods)' /proc/1/cgroup
  echo "end output"
else
  echo "Not running inside Docker"
fi

echo "--- uname -a"
if command -v uname >/dev/null 2>&1; then
  uname -a
else
  echo "uname command unavailable"
fi
echo ""

echo "--- lsb_release -a"
if command -v lsb_release >/dev/null 2>&1; then
  lsb_release -a
else
  echo "lsb_release command unavailable"
fi
echo ""

CMD="$(hab pkg path core/rust/"$toolchain")/bin/cargo-clippy"
readonly CMD
if command -v ldd >/dev/null 2>&1; then

  echo "--- ldd --version"
  ldd --version
  echo ""

  LDD_TARGET="$(hab pkg path core/glibc)/lib/libc.so.6"
  echo "--- ldd $LDD_TARGET"
  ldd "$LDD_TARGET" || true
  echo ""

  echo "--- ldd $CMD"
  ldd "$CMD" || true
  echo ""

else
  echo "ldd command unavailable"
fi
echo ""

echo "--- Running clippy!"
command="$CMD clippy --all-targets -- ${clippy_args[*]}"
# if ${BUILDKITE:-false}; then
#   command="LD_PRELOAD=$(hab pkg path core/glibc)/lib/libc.so.6 $command"
# fi
echo "Clippy rules: $command"
eval "$command"
