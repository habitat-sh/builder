# System Overview

Habitat Builder is the public SaaS control plane for Habitat. This repository contains the backend services, shared Rust crates, the web UI, and supporting tooling used to serve the `hab` client and the Builder web experience.

## Languages and technologies

- **Rust** drives the backend services, shared libraries, and standalone CLI tooling.
- **TypeScript / Angular** powers the web UI in `components/builder-web`.
- **SCSS** is used for frontend styling.
- **Shell scripts** and **Terraform** support local development, CI, and infrastructure workflows.

## Main runtime pieces

- **`components/builder-api`**: primary Rust HTTP API gateway (`bldr-api`) built on Actix Web.
- **`components/builder-web`**: Angular/TypeScript single-page app for the Builder UI.
- **`components/builder-api-proxy`**: production-facing proxy that serves the UI assets alongside API-facing traffic.
- **`components/builder-core` / `components/builder-db` / `components/builder-protocol`**: shared core logic, persistence layer, and protocol/message definitions used across services.
- **Client integrations**: `github-api-client`, `oauth-client`, and `artifactory-client`.
- **Infrastructure/supporting services**: Postgres-backed data access, memcache, MinIO/object storage, Terraform, support scripts, and test fixtures.

## Repository shape

- **Rust workspace members** are declared in the root `Cargo.toml`.
- **Frontend assets** live under `components/builder-web`.
- **Developer environment docs** live under `dev-docs/`.
- **Support scripts and CI helpers** live under `support/` and `test/`.

## Main entry points

- **Backend API binary**: `components/builder-api/src/main.rs` (`bldr-api`)
- **Web UI app**: `components/builder-web` via `npm start` and `npm run build`
- **Standalone CLI**: `tools/token-generator/src/main.rs` (`token-generator`)

## Development topology

The documented development flow is:

1. Run Builder services inside a Linux VM / Habitat Studio environment.
2. Optionally run `components/builder-web` on the host for UI work.
3. Reach the API locally at `http://localhost:9636/v1/status`.

This split keeps the backend close to the supported Linux runtime while allowing faster local UI iteration.

## Test approach

- **Rust unit tests** run through the root `Makefile` (`make unit`), which delegates to `test/run_cargo_test.sh` and `cargo test`.
- **Rust functional tests** run with `make functional`, using `cargo test --features functional` for component-level functional coverage.
- **Web UI tests** run from `components/builder-web` with `npm test`; unit coverage is driven by Karma/Jasmine.
- **Broader environment verification** is documented through the Habitat Studio workflow, including `test-builder` for API-level validation.
