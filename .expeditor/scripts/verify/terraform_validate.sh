#!/bin/bash

set -euo pipefail

# In the absence of a real Terraform parser, we can use a bit of awk
# to extract the required version from a file.
#
# Given a file with contents like this:
#
#   terraform {
#     required_version = "0.12.13"
#   }
#
# we would extract `0.12.13` (without quotes!).
terraform_version() {
    versions_file="${1}"
    awk 'BEGIN { FS=" *= *"} /required_version/ {gsub("\"","",$2); print $2}' "${versions_file}"
}

readonly terraform_version="$(terraform_version terraform/versions.tf)"
readonly terraform_artifact="terraform_${terraform_version}_linux_amd64.zip"

# Install Terraform
(
    # We do this so we don't have to contend with the binary and
    # directory names (both "terraform") conflicting.
    mkdir bin
    cd bin
    curl -O "https://releases.hashicorp.com/terraform/${terraform_version}/${terraform_artifact}"
    # leaves a `terraform` binary in the current directory
    unzip "${terraform_artifact}"
)

# Validate the terraform directory
./bin/terraform init terraform
./bin/terraform validate terraform
