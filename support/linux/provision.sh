#!/bin/bash
set -eux

curl https://raw.githubusercontent.com/habitat-sh/habitat/master/components/hab/install.sh | sudo bash
sudo hab install core/busybox-static core/hab-studio
sudo hab install \
  core/direnv \
  core/wget \
  core/docker \
  core/curl -b
# shellcheck disable=SC2016
echo 'eval "$(direnv hook bash)"' >> ~/.bashrc
