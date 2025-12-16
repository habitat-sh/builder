# Token Generator

A standalone CLI tool that generates user authentication tokens for Habitat Builder.

## Overview

The Token Generator is a utility tool that mimics the functionality of Habitat Builder token provisioning. It allows administrators and developers to generate authentication tokens for specific user accounts using the Builder's signing key infrastructure.

## Purpose

This tool is designed for:
- Administrative token generation for user accounts
- Development and testing environments
- Service account authentication setup
- Manual token provisioning when oauth flows are unavailable

## Installation

```bash
hab pkg install habitat/builder-token-generator --channel LTS-2024
```

## Usage

### Basic Syntax

```bash
hab pkg exec habitat/builder-token-generator token-generator --account-id <ID> --key-path <PATH> [OPTIONS]
```

### Required Arguments

- `--account-id, -a <ID>`: The account ID for which to generate the token
- `--key-path, -k <PATH>`: Path to the directory that contains the Builder signing key file (typically `/hab/svc/builder-api/files`)

### Optional Arguments

- `--verbose, -v`: Enable verbose logging output
- `--help, -h`: Display help information
- `--version, -V`: Display version information

### Examples

#### Generate a token for account 12345

```bash
hab pkg exec habitat/builder-token-generator token-generator --account-id 12345 --key-path /hab/svc/builder-api/files
```
