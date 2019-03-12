#!/bin/bash

set -eux

# Add a very uniquely named user/group pair for worker builds to run under.
sudo useradd --groups=tty --create-home krangschnak || echo "User 'krangschnak' already exists"

# Install docker via apt-get for now until we hammer out the
# steps with the hab package
sudo apt-get update
sudo apt-get -y install docker.io
