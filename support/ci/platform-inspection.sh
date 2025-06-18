#!/bin/bash

# This was written to inspect the context that run_clippy was executing under
# during verify pipeline while debugging segfaults due to version misalignments
# with glibc and other libraries
#
# One word of caution, the ldd --version statement is dependent on the runtime
# environment at the time it's called. If you're managing that environment you
# will want to do another ldd --version call prior to the environment mgmt.
#
# Also, it's still purpose built for the run_clippy case but it can be extended
# in the future if needed.

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

# shellcheck source=./shared.sh
source ./support/ci/shared.s
toolchain=$(get_toolchain)
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
