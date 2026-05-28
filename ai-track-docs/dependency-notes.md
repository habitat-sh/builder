# Dependency Notes

## Scope

These notes focus on the low-risk `tools/token-generator` module and document the most important dependencies to watch before making small changes.

## Critical dependencies

| Dependency | Current declaration | Resolved version/source | Why it matters |
| --- | --- | --- | --- |
| `builder_core` | `path = "../../components/builder-core"` | in-repo path dependency | Provides `AccessToken` and feature-flag behavior used for token generation. |
| `habitat_core` | `git = "https://github.com/habitat-sh/habitat.git"` | git commit `aff457eb48ae3894c83dca2afbbe3a886296a92c` via `Cargo.lock` | External core dependency; release builds in this environment also surfaced the `SSL_CERT_FILE` build-script requirement. |
| `clap` | `version = "4"` | `4.6.0` | Defines CLI parsing and user-facing argument behavior. |
| `anyhow` | `1.0` | `1.0.102` | Shapes error propagation and error text shown by the tool. |
| `log` | `0.4` | `0.4.44` | Shared logging facade used in both normal runs and helper logic. |
| `env_logger` | `0.10` | `0.10.2` | Controls runtime logging initialization and output defaults. |

## Minimal pinning / constraint proposals

These are **proposals only**. They narrow dependency drift without introducing major-version upgrades.

| Dependency | Proposed constraint | Rationale |
| --- | --- | --- |
| `clap` | `>=4.6, <4.7` | Keeps the CLI on the currently resolved minor line while avoiding broader `4.x` drift. |
| `anyhow` | `>=1.0.102, <1.1` | Keeps error-handling behavior on the current stable major line with a concrete floor at the resolved version. |
| `log` | `>=0.4.44, <0.5` | Preserves the existing logging facade contract and avoids any future major break. |
| `env_logger` | `>=0.10.2, <0.11` | Keeps logger initialization behavior within the current minor line. |
| `habitat_core` | add `rev = "aff457eb48ae3894c83dca2afbbe3a886296a92c"` | Makes the manifest match the lockfile's actual external source and avoids silent branch drift. |

## Notes for future changes

- Prefer pinning the external git dependency (`habitat_core`) first; it has the highest supply-chain and reproducibility impact for this module.
- `builder_core` is already workspace-local, so its stability comes from the repository branch rather than an external registry version.
- If dependency constraints are tightened later, re-run:

```bash
cargo build -p token-generator
cargo test -p token-generator
```

- For release-mode checks in this environment, remember the observed `SSL_CERT_FILE=/etc/ssl/cert.pem` requirement when building transitive `habitat_core`.
