#!/bin/bash

source ../plan.sh
pkg_exposes=()
do_install() {
  build_line "Copying new config into package"
  cp -v "../default.toml" "$pkg_prefix/default.toml"
  cp -v "../config/pg_hba.conf" "$pkg_prefix/config/pg_hba.conf"
  cp -v "../config/pg_ident.conf" "$pkg_prefix/config/pg_ident.conf"
  cp -v "../config/postgresql.conf" "$pkg_prefix/config/postgresql.conf"

  build_line "Copying run hooks into package"
  for hook in ../hooks/*; do
    cp -v "$hook" "$pkg_prefix/hooks/$(basename "$hook")"
  done
}
