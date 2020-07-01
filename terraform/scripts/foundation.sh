#!/bin/bash

# Various low-level foundational work needed to set up Habitat for our
# Builder environment.

set -eux

# Set up the filesystem
sudo mkfs.ext4 /dev/nvme1n1
sudo mount /dev/nvme1n1 /mnt
echo '/dev/nvme1n1 /mnt     ext4   defaults 0 0' | sudo tee -a /etc/fstab
sudo mkdir -p /mnt/hab
sudo ln -s /mnt/hab /hab

until [[ -f /var/lib/cloud/instance/boot-finished ]]; do
  sleep 1
done

# set our locale
sudo localectl set-locale LANG=en_US.UTF-8

sudo apt-get update
sleep 10
sudo apt-get -y install ntpdate
sudo apt-get -y install git
sudo crontab -l | { cat; echo "0 0 1 * * ntpdate time.google.com"; } | sudo crontab -

# Add hab user / group
sudo adduser --group hab || echo "Group 'hab' already exists"
sudo useradd -g hab hab || echo "User 'hab' already exists"
