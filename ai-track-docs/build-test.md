# Build and Test

## Environment baseline

The repo's primary development documentation points to a Linux VM / Habitat Studio setup for backend work, with optional host-side web UI development on macOS. See `dev-docs/dev-environment.md` for the full flow.

## Rust workspace and service commands

From the repository root:

| Command | Purpose | Notes |
| --- | --- | --- |
| `cargo build -p token-generator` | Build the standalone token generator CLI | Good low-risk module check from repo root |
| `cargo test -p token-generator` | Run unit tests for the token generator | Fast, isolated validation |
| `make build` | Build Rust library and service components | Linux-only target via `make linux` |
| `make unit` | Run unit test suites | Wraps `test/run_cargo_test.sh` |
| `make functional` | Run functional test suites | Uses `cargo test --features functional` |
| `make test` | Alias for full functional test pass | |
| `make lint` | Run clippy-based linting | Uses repo lint allow/deny lists |
| `make fmt` | Run rustfmt helper script | `support/ci/rustfmt.sh` |

Inside the Habitat Studio, the documented service workflow is:

- `start-builder` to boot services
- `build-builder` for a full backend rebuild
- `build-builder api` for an incremental API-only rebuild
- `test-builder` for automated API verification

## Recommended low-risk module workflow

For small, isolated changes, `tools/token-generator` is a good starting point because it is a standalone CLI with local unit tests.

From the repository root:

```bash
cargo build -p token-generator
cargo test -p token-generator
```

This validates the chosen module without rebuilding the full backend stack.

## Web UI commands

From `components/builder-web`:

| Command | Purpose |
| --- | --- |
| `npm install` | Install UI dependencies |
| `npm start` | Run the development server on port 3000 |
| `npm run build` | Build JS and CSS assets |
| `npm test` | Run unit + e2e test flow |
| `npm run test-unit` | Run Karma/Jasmine unit tests |

The checked-in `package.json` currently expects **Node >= 20** and **npm >= 10**.

For focused backend changes, prefer targeted `cargo build -p <crate>` and `cargo test -p <crate>` before running broader repo-level commands.

## Quick validation points

- API health: `curl -v http://localhost:9636/v1/status`
- UI dev URL: `http://localhost:3000/#/pkgs`
- Root repo overview: `README.md`
- Full environment setup: `dev-docs/dev-environment.md`
