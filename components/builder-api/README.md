# builder-api

Habitat-Builder HTTP API gateway

## Feature flags

`builder-api` already has a startup-time feature-flag mechanism in
`components/builder-api/src/server/mod.rs`. Flags are declared in the `feat` module,
loaded from `api.features_enabled`, and then checked at runtime with `feat::is_enabled(...)`.

### Example: `ARTIFACTORY`

`ARTIFACTORY` is an existing flag that switches package upload and delete behavior in
`components/builder-api/src/server/resources/pkgs.rs`:

- **On:** package uploads and deletes go through the Artifactory client
- **Off:** the same paths fall back to the S3 package store

### Flag lifecycle

1. **Declare** the flag in `components/builder-api/src/server/mod.rs` inside the `features!` block.
2. **Configure** it in the Builder API config as `api.features_enabled`.
3. **Enable at startup** when `enable_features(&config)` maps the configured string to a known flag.
4. **Guard behavior** with `feat::is_enabled(...)` in the runtime code path.
5. **Disable** it by removing the flag from `api.features_enabled` and restarting `builder-api`.

Only flags present at process startup are enabled; this is not a live runtime toggle. Unknown
entries are ignored and now logged as warnings during startup.

### Toggle example

Turn the Artifactory-backed package flow **on**:

```toml
[api]
features_enabled = "ARTIFACTORY"
```

Turn it **off** by removing the flag or leaving the list empty:

```toml
[api]
features_enabled = ""
```

## Verifying paginated response tracing

Run `builder-api` with `RUST_LOG=builder_api=trace`, then hit a paginated endpoint such as
`/v1/depot/channels/core/stable/pkgs?range=0`. Each response serialized through
`package_results_json` will emit a trace line like
`Serializing paginated response: items=50, total_count=123, range_start=0, range_end=49`.
