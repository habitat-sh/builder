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
- `STRICT_EXT_REGISTRY_HTTPS`

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

### Safety switch: strict extension registry HTTPS

`STRICT_EXT_REGISTRY_HTTPS` gates a higher-risk behavior change for custom
registry URLs:

- **OFF**: explicit custom `http://...` registry URLs are still forwarded for
  compatibility.
- **ON**: explicit custom URLs must parse as `https://...`; non-HTTPS custom
  URLs are rejected with `400 Bad Request`.
- The built-in Docker default remains `https://hub.docker.com/v2` in both
  modes.

This flag is intended as a rollout/rollback switch for tightening outbound
credential validation without breaking existing internal HTTP registries all at
once.

## Risk notes

- **Credential handling:** this endpoint forwards raw credentials to an upstream
  registry login endpoint. Do not log request bodies or persist the password in
  handler debug output.
- **Registry defaults:** the Docker Hub fallback is hard-coded. Adding another
  registry type requires both code and doc updates so callers do not assume
  unsupported defaults.
- **Flagged rollout:** enabling `STRICT_EXT_REGISTRY_HTTPS` can break callers
  that still rely on explicit non-HTTPS registry URLs. Roll it out only after
  validating the affected integrations.
- **Error semantics:** transport failures still map to authorization-style
  errors for compatibility. If you change that mapping, coordinate any UI/API
  consumers that interpret `403` specially.
- **Doc-with-code rule:** changes to `/v1/ext` behavior, feature flags, or
  supported registry types should update this README in the same branch/PR.

## Validation

For focused local validation of the extension helper logic:

```bash
cargo check -p habitat_builder_api --tests
```

ON/OFF validation for the safety switch should cover both compatibility paths:

1. With `STRICT_EXT_REGISTRY_HTTPS` **disabled**, confirm an explicit
   `http://...` registry URL is still accepted by the helper path/tests.
2. With `STRICT_EXT_REGISTRY_HTTPS` **enabled**, confirm the same explicit
   `http://...` URL is rejected with `400 Bad Request`.
3. Confirm the Docker default URL and explicit `https://...` URLs continue to
   work in both modes.

Telemetry is available through the Builder API metrics surface:

- `ext-registry.validation`
- `ext-registry.insecure-url.allowed`
- `ext-registry.insecure-url.blocked`

## Resources-folder Clippy strictness

`src/server/resources/` now opts into `clippy::manual_let_else` at the module
boundary in `src/server/resources/mod.rs`.

This scope is intentionally narrow:

- the folder contains many request handlers that share the same
  early-return-on-auth-or-parse-failure shape
- `let ... else` keeps those guard clauses uniform without enabling a new lint
  across unrelated startup and middleware code
- handlers already keep targeted `#[allow(clippy::needless_pass_by_value)]`
  suppressions because Actix extractors are passed by value; those suppressions
  remain valid and are not part of this sweep

Use a targeted `#[allow(clippy::manual_let_else)]` only when the `match` form is
materially clearer, typically because the error arm needs richer logging,
structured response construction, or shared cleanup before returning.

For repeatable autofix runs on this folder:

```bash
./support/ci/fix_builder_api_resources_clippy.sh
```

Then validate the scope with:

```bash
cargo clippy -p habitat_builder_api --all-targets --tests
```

## Contract tests

This component now has focused contract coverage for two request boundaries:

| Boundary | File | Contract covered |
| --- | --- | --- |
| Extension credential validation | `src/server/resources/ext.rs` | required credentials, Docker default URL, explicit URL override, unsupported registry rejection, and outbound JSON payload shape |
| Request target inference | `src/server/helpers.rs` | missing `User-Agent` fallback, valid Habitat target parsing, and invalid-target fallback to Linux |

### Update process

When a boundary changes, update the contract tests in the same branch/PR:

1. Identify the externally visible rule that changed: request fields, fallback behavior, status mapping, or header parsing.
2. Update the narrowest unit or handler-level test near that boundary (`ext.rs` or `helpers.rs` today).
3. Refresh this README section if the supported input, fallback, or error semantics changed.
4. Run at least `cargo check -p habitat_builder_api --tests`; if the local environment links successfully, also run the focused test target (`cargo test -p habitat_builder_api ext::` or the relevant helper test).

## API TODO and edge sweep

Reviewed TODO/FIXME hotspots under `src/server/` and left the current scope intentionally narrow.

| Area | Current edge / TODO | Rationale for leaving as-is in this update |
| --- | --- | --- |
| `helpers.rs` | `target_from_headers` is still a compatibility fallback based on `User-Agent` | This behavior is still referenced by package/channel handlers. The safe change here was to lock the current contract with tests before a future deprecation. |
| `helpers.rs` | helper module is still a mixed “grab bag” | Breaking it up is worthwhile, but it is structural refactoring across many handlers rather than a doc-with-code boundary fix. |
| `resources/pkgs.rs` and `resources/channels.rs` | repeated “deprecate target from headers” TODOs | These call sites depend on the same compatibility rule. Documenting and testing the shared helper gives one contract anchor without rewriting multiple handlers now. |
| `resources/pkgs.rs` | async/provider-model TODOs around upload and Artifactory/S3 handling | Those are larger transport/storage changes with broader operational risk than this task’s contract-test scope. |
| `resources/origins.rs` | file remains large and has invitation/key-management TODOs | These are domain-shape issues, not currently ambiguous request/response boundaries. They should be addressed in a dedicated decomposition effort. |
| `services/s3.rs` | blocking call inside async code | Important, but it needs an async storage migration rather than a boundary-contract update; changing it here would be higher risk than validating current API behavior. |
| `mod.rs` | legacy compatibility TODO tied to an upgrade path | This is a startup/runtime compatibility note rather than a request boundary. No behavior changed here. |

The guiding rule for this sweep was: **validate stable boundary behavior first, defer structural rewrites until the compatibility-dependent call sites are ready to move together.**
