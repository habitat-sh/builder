#!/bin/bash

if [ ! -f /bin/hab ]; then
  sudo useradd -r -U hab
  curl https://raw.githubusercontent.com/habitat-sh/habitat/master/components/hab/install.sh | sudo bash
fi