# builder-api

Habitat Builder's HTTP API gateway. This service fronts the REST API used by the
web UI and `hab` clients, wires shared middleware, and coordinates access to the
database, OAuth providers, package storage, memcache, and selected upstream
services.

## Subsystem overview

Key entry points live under `src/server/`:

| Area | Purpose |
| --- | --- |
| `mod.rs` | Boots the Actix server, wires middleware, registers routes, and enables feature flags. |
| `framework/` | Shared middleware such as authentication/session handling. |
| `resources/` | Route handlers grouped by API surface (`origins`, `pkgs`, `jobs`, `ext`, and others). |
| `services/` | Integrations used by handlers, including S3/object storage and memcache access. |
| `helpers.rs` | Shared response/pagination/header helpers used across handlers. |

## Feature flags

`builder-api` enables selected routes and behavior at startup through
`api.features_enabled`. Known flags are declared in `src/server/mod.rs`.

Current flags include:

- `LIST`
- `LEGACYPROJECT`
- `ARTIFACTORY`
- `BUILDDEPS`

Unknown flags are ignored unless explicitly mapped in `enable_features`, so keep
README guidance and `mod.rs` in sync when adding or removing flags.

## Extension guidance: `/v1/ext`

The `resources/ext.rs` module currently exposes:

- `POST /v1/ext/integrations/{registry_type}/credentials/validate`

This endpoint is intended for extension-style integration flows where the UI
needs to validate registry credentials before saving integration settings.

### Expected request body

```json
{
  "username": "registry-user",
  "password": "registry-password",
  "url": "https://registry.example.test"
}
```

- `username` and `password` are required.
- `url` is optional only for `registry_type=docker`, where Builder falls back to
  `https://hub.docker.com/v2`.
- Any other registry type must provide an explicit `url` until a registry-specific
  default and validation flow are added in code.

### Current behavior

1. The caller must have a valid Builder session.
2. Builder forwards the JSON payload to `{url}/users/login`.
3. Upstream `200 OK` returns `200 OK`.
4. Missing credentials, unsupported registry types without a URL, or non-`200`
   upstream responses return `400 Bad Request`.
5. Transport failures currently return `403 Forbidden` via `Error::Authorization`
   to preserve existing behavior.

## Risk notes

- **Credential handling:** this endpoint forwards raw credentials to an upstream
  registry login endpoint. Do not log request bodies or persist the password in
  handler debug output.
- **Registry defaults:** the Docker Hub fallback is hard-coded. Adding another
  registry type requires both code and doc updates so callers do not assume
  unsupported defaults.
- **Error semantics:** transport failures still map to authorization-style
  errors for compatibility. If you change that mapping, coordinate any UI/API
  consumers that interpret `403` specially.
- **Doc-with-code rule:** changes to `/v1/ext` behavior, feature flags, or
  supported registry types should update this README in the same branch/PR.

## Validation

For focused local validation of the extension helper logic:

```bash
cargo test -p habitat_builder_api ext::
```
