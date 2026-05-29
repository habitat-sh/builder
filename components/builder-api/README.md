# builder-api

Habitat-Builder HTTP API gateway

## Verifying paginated response tracing

Run `builder-api` with `RUST_LOG=builder_api=trace`, then hit a paginated endpoint such as
`/v1/depot/channels/core/stable/pkgs?range=0`. Each response serialized through
`package_results_json` will emit a trace line like
`Serializing paginated response: items=50, total_count=123, range_start=0, range_end=49`.
