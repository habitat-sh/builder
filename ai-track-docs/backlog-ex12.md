# Backlog

No tracker integration is available in this session, so these backlog items are captured here for follow-up work.

## 1. Enable repository-level build and unit-test execution in GitHub Actions

**Why**
- The GitHub Actions PR workflow currently delegates secret scanning and SBOM generation, but `build` and `unit-tests` are disabled in the checked-in workflow stub.

**Acceptance criteria**
- GitHub Actions PR runs execute at least one repository-supported build/test path.
- The selected path is documented, including expected runtime and required secrets or environment.
- A failing unit test causes the PR workflow to fail.

**Code links**
- `.github/workflows/ci-main-pull-request-checks.yml`
- `.expeditor/verify.pipeline.yml`
- `test/run_cargo_test.sh`

## 2. Pin `habitat_core` and narrow token-generator dependency drift

**Why**
- `tools/token-generator/Cargo.toml` pulls `habitat_core` from Git without a pinned `rev`, while the lockfile already resolves to a specific commit.
- The module also uses broad semver declarations (`clap = "4"`, `anyhow = "1.0"`), which allow drift beyond the currently validated minor versions.

**Acceptance criteria**
- `tools/token-generator/Cargo.toml` pins `habitat_core` to a specific `rev`.
- Critical direct dependencies for `token-generator` use explicit low-risk constraints aligned with currently resolved versions.
- `cargo build -p token-generator` and `cargo test -p token-generator` still pass after the manifest update.

**Code links**
- `tools/token-generator/Cargo.toml`
- `Cargo.lock`
- `ai-track-docs/dependency-notes.md`

## 3. Add CI-safe release benchmark wrapper for token-generator

**Why**
- The micro-benchmark exists, but release-mode execution depends on `SSL_CERT_FILE=/etc/ssl/cert.pem` in this environment.
- That requirement is documented, but not yet encoded in a reusable command or script.

**Acceptance criteria**
- A single documented command or script runs the benchmark reliably in local and CI-like environments.
- The script sets any required environment variables explicitly.
- Benchmark output is easy to capture for regression tracking.

**Code links**
- `tools/token-generator/src/main.rs`
- `ai-track-docs/token-generator-benchmark.md`
- `ai-track-docs/build-test.md`

## 4. Expand secret-sample and doc audit beyond the current sample env file

**Why**
- One concrete secret-like sample was remediated, but other docs and config examples still deserve a repo-wide placeholder audit.
- Secret hygiene relies on documentation consistency as much as ignore rules.

**Acceptance criteria**
- Secret-bearing samples and docs use placeholders instead of live-looking credentials.
- Any intentionally checked-in test fixtures are clearly distinguished from real secrets.
- Security guidance links the most relevant local-secret locations and file-permission expectations.

**Code links**
- `.secrets/habitat-env.sample`
- `tools/hab_token/README.md`
- `ai-track-docs/security-notes.md`
- `dev-docs/dev-environment.md`

## 5. Standardize structured logging conventions for small CLI tools

**Why**
- `token-generator` now emits structured fields (`op`, `status`, `elapsed_ms`) on important completion paths, but that convention is not yet generalized.
- Similar lightweight tools would benefit from a shared logging shape and doc reference.

**Acceptance criteria**
- A short logging convention doc exists for CLI tools and defines required fields.
- At least one additional tool or script adopts the same structured-field pattern.
- Log-viewing guidance is documented in one central place and linked from tool-specific docs.

**Code links**
- `tools/token-generator/src/main.rs`
- `tools/token-generator/readme.md`
- `ai-track-docs/logging.md`
