#!/bin/bash

set -euo pipefail

# TODO: sync this with terraform/versions.tf
readonly terraform_version=0.12.13
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
./bin/terraform validate terraform
