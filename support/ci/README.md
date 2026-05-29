# CI reliability notes

This directory contains CI helper scripts and workflow-adjacent tooling.

## Reliability changes

The local GitHub Actions workflows and composite actions now use bounded retry,
timeout, cache, and concurrency controls for the flakiest repo-owned paths:

- `rust-cargo-audit-check.yml`
  - caches the pinned `cargo-audit` binary
  - retries `cargo install cargo-audit` on cache misses only
  - cancels superseded runs on the same ref
  - bounds the job with a 15 minute timeout
- `hab-pkg-build-upload.yaml`
  - keeps the existing component matrix
  - adds per-component concurrency so the same component is not published by two
    runs at once
  - bounds the packaging job with a 75 minute timeout
- `adhoc-hab-pkg-build-upload.yaml`
  - prevents overlapping manual builds of the same component
  - bounds the packaging job with a 75 minute timeout
- composite Habitat actions
  - retry only network-sensitive install, key-download, and upload operations
  - bound `hab pkg build`, `hab pkg install`, and `hab pkg upload` with timeouts
  - do **not** retry `hab pkg build`, so deterministic build failures still fail
    fast

## Evidence

Sampled on `2026-05-29` with:

```bash
GH_PAGER=cat gh api /repos/habitat-sh/builder/actions/runs?per_page=20 | jq -r \
  '.workflow_runs[] | select(.name=="Rust Cargo Audit") | [.run_started_at,.updated_at] | @tsv'
```

Observed `Rust Cargo Audit` durations from 11 recent successful runs:

- average: `182.2s`
- min: `137s`
- max: `197s`

That made the repeated `cargo install cargo-audit` step a good candidate for
binary caching and cache-miss-only retries.

## Validation

Validate the workflow and action YAML parses cleanly:

```bash
python3 - <<'PY'
from pathlib import Path
import yaml

paths = [
    Path('.github/workflows/rust-cargo-audit-check.yml'),
    Path('.github/workflows/hab-pkg-build-upload.yaml'),
    Path('.github/workflows/adhoc-hab-pkg-build-upload.yaml'),
    Path('.github/actions/hab-install-linux/action.yaml'),
    Path('.github/actions/hab-pkg-build-and-upload-linux/action.yaml'),
]
for path in paths:
    yaml.safe_load(path.read_text())
    print(f'parsed {path}')
PY
```

## Rollback

If any workflow proves too strict or a retry/timeout default is too aggressive:

1. Revert the affected workflow or action file.
2. Re-run the YAML parse validation above.
3. Re-run the corresponding GitHub Actions workflow to confirm the previous
   behavior is restored.
