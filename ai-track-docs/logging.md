# Logging

## Viewing logs

The token generator uses `env_logger` and writes logs to stderr.

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

## Structured fields

Structured completion logs use the fields `op`, `status`, and `elapsed_ms`, for example:

```text
op=validate_args status=ok elapsed_ms=0
op=generate_token status=ok elapsed_ms=3
```

## Toggle

Set `TOKEN_GENERATOR_STRUCTURED_LOGS=off` to disable the structured completion logs for local troubleshooting without changing token generation behavior:

```bash
TOKEN_GENERATOR_STRUCTURED_LOGS=off \
cargo run -p token-generator -- --account-id 12345 --key-path /hab/svc/builder-api/files
```
