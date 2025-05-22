#!/bin/bash
set -eux

curl https://raw.githubusercontent.com/habitat-sh/habitat/master/components/hab/install.sh | sudo bash
sudo hab install --channel stable --binlink core/direnv core/hab-studio
sudo hab install --channel LTS-2024 --binlink \
  core/busybox-static \
  core/wget \
  core/docker \
  core/curl
# shellcheck disable=SC2016
echo 'eval "$(direnv hook bash)"' >> ~/.bashrc
