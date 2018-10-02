# Habitat Token Helper

This script is a simple helper for managing multiple Habitat personal access tokens.
It makes is easier to switch between tokens during development / acceptance testing / production usage.

## Prerequisites

1. Copy the `hab-token` script to a directory in your path - e.g., `~/bin`
2. Create a new file called `.tokens` in your home directory - e.g. '~/.tokens'

The `.tokens` file should have the following format:

```
#!/bin/bash
TOKEN_LIVE=<your prod token>
TOKEN_ACCEPTANCE=<your acceptance token>
TOKEN_DEV=<your dev token>
```

## Usage

When you want to switch to a specific token, issue a command like the following in your shell:

```
eval $(hab-token live)
```

This should set and export the `HAB_AUTH_TOKEN` env variable appropriately.  You can confirm that the token is set by doing the following:

```
echo $HAB_AUTH_TOKEN
```
