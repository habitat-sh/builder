#!/bin/bash

source ../plan.sh
do_install() {
  local pkg_path
  pkg_path=$(hab pkg path habitat/"$pkg_name")

  build_line "Copying new config into package"
  cp -v "../default.toml" "$pkg_path/default.toml"
  cp -v "../config/pg_hba.conf" "$pkg_path/config/pg_hba.conf"
  cp -v "../config/pg_ident.conf" "$pkg_path/config/pg_ident.conf"
  cp -v "../config/postgresql.conf" "$pkg_path/config/postgresql.conf"

  build_line "Copying run hooks into package"
  for hook in ../hooks/*; do
    cp -v "$hook" "$pkg_path/hooks/$(basename "$hook")"
  done
}
