#!/bin/bash
# Script will install Cloudwatch agent and Automx agent
sudo apt install awscli -y
wget https://s3.amazonaws.com/amazoncloudwatch-agent/ubuntu/amd64/latest/amazon-cloudwatch-agent.deb -O /tmp/amazon-cloudwatch-agent.deb
sudo dpkg -i -E /tmp/amazon-cloudwatch-agent.deb
sudo systemctl enable amazon-cloudwatch-agent
sudo systemctl start amazon-cloudwatch-agent
wget https://console.automox.com/installers/amagent_latest_amd64.systemd.deb -O /tmp/amagent_latest_amd64.deb
sudo dpkg -i -E /tmp/amagent_latest_amd64.deb
/opt/amagent/amagent --setkey $AutomoxKey
/opt/amagent/amagent --setgrp \"Default Group/Builder\"
systemctl restart amagent
sleep 10
systemctl stop amagent
sleep 10
/opt/amagent/amagent --deregister
systemctl start amagent