# Token Generator

A standalone CLI tool that generates user authentication tokens for Habitat Builder.

See [../../ai-track-docs/extending-token-generator.md](../../ai-track-docs/extending-token-generator.md) for notes on making small, safe changes to this module.

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

The tool resolves `--key-path` before token generation and reports inaccessible paths with an explicit `Unable to access key path: ...` error.

### Optional Arguments

- `--verbose, -v`: Enable verbose logging output
- `--help, -h`: Display help information
- `--version, -V`: Display version information

### Examples

#### Generate a token for account 12345

```bash
hab pkg exec habitat/builder-token-generator token-generator --account-id 12345 --key-path /hab/svc/builder-api/files
```

## Viewing logs

The tool uses `env_logger` and writes logs to stderr.

```bash
cargo run -p token-generator -- --account-id 12345 --key-path /hab/svc/builder-api/files
```

Use `--verbose` to include debug output:

```bash
cargo run -p token-generator -- --account-id 12345 --key-path /hab/svc/builder-api/files --verbose
```

To save logs to a file while keeping the token on stdout:

```bash
cargo run -p token-generator -- --account-id 12345 --key-path /hab/svc/builder-api/files \
  2>token-generator.log
```

Structured completion logs now use the fields `op`, `status`, and `elapsed_ms`, for example:

```text
op=validate_args status=ok elapsed_ms=0
op=generate_token status=ok elapsed_ms=3
```

Set `TOKEN_GENERATOR_STRUCTURED_LOGS=off` to disable those structured completion logs while keeping the rest of the tool behavior unchanged:

```bash
TOKEN_GENERATOR_STRUCTURED_LOGS=off \
cargo run -p token-generator -- --account-id 12345 --key-path /hab/svc/builder-api/files
```
