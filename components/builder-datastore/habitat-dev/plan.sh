#!/bin/bash

source ../plan.sh
do_install() {
  local pkg_path
  pkg_path=$(hab pkg path habitat/"$pkg_name")

  build_line "Copying new config into package"
  cp -v "../default.toml" "$pkg_path/default.toml"
  cp -v "../config/pg_hba.toml" "$pkg_path/config/pg_hba.toml"
  cp -v "../config/pg_ident.toml" "$pkg_path/config/pg_ident.toml"
  cp -v "../config/postgresql.toml" "$pkg_path/config/postgresql.toml"

  build_line "Copying run hooks into package"
  for hook in ../hooks/*; do
    cp -v "$hook" "$pkg_path/hooks/$(basename "$hook")"
  done
}
