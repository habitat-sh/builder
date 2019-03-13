#!/bin/bash

set -eux

# Install docker via apt-get for now until we hammer out the
# steps with the hab package
sudo apt-get update
sudo apt-get -y install docker.io
