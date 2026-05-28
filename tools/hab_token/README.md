# Habitat Token Helper

This script is a simple helper for managing multiple Habitat personal access tokens.
It makes is easier to switch between tokens during development / acceptance testing / production usage.

## Prerequisites

1. Copy the `hab-token` script to a directory in your path - e.g., `~/bin`
2. Create a new file called `.tokens` in your home directory - e.g. '~/.tokens'
3. Restrict that file to your user only, for example `chmod 600 ~/.tokens`

The `.tokens` file should have the following format:

```shell
#!/bin/bash
TOKEN_LIVE=<your prod token>
TOKEN_ACCEPTANCE=<your acceptance token>
TOKEN_DEV=<your dev token>
```

Keep `.tokens` outside the repository and never commit it or copy its contents into tracked sample files.

## Usage

When you want to switch to a specific token, issue a command like the following in your shell:

```shell
eval $(hab-token live)
```

This should set and export the `HAB_AUTH_TOKEN` env variable appropriately.  You can confirm that the token is set by doing the following:

```shell
echo $HAB_AUTH_TOKEN
```
