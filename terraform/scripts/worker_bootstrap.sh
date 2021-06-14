#!/bin/bash

set -eux

# Install docker via apt-get for now until we hammer out the
# steps with the hab package
sudo apt-get update
sudo apt-get -y install docker.io
sudo systemctl start docker
sleep 10
sudo systemctl stop docker
sudo mv /var/lib/docker /mnt/docker
sudo ln -s /mnt/docker /var/lib/docker
sudo systemctl start docker
sudo systemctl enable docker
