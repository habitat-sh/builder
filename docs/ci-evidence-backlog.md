# CI Evidence Backlog

## Epic: Strengthen lightweight CI evidence

Improve the quality of lightweight CI evidence so coverage, contract checks, and shell scanning are easier to trust and cheaper to review.

### Outcomes
- builder-web coverage evidence becomes actionable instead of mostly failure-tail output
- builder-api contract coverage expands around stable JSON boundaries
- shell scanning stays fast, repo-scoped, and reviewable
- focused workflows publish consistent evidence without blocking merges by default

### Acceptance
- at least 3 follow-up items are tracked with clear acceptance criteria and links
- each item targets one bounded improvement that can land independently
- the epic captures progress and cross-links the related workflows and PRs

### Links
- PR #1994: https://github.com/habitat-sh/builder/pull/1994
- Shell scanning workflow: `./.github/workflows/validate-shell-scripts.yml`
- Builder-web coverage summary workflow: `./.github/workflows/post-builder-web-coverage-summary.yml`
- Builder-api contract workflow: `./.github/workflows/validate-builder-api-contracts.yml`

## Backlog Items

### 1. Stabilize builder-web coverage evidence

Make the non-blocking summary workflow report real totals more often instead of only fallback evidence.

#### Acceptance
- `npm run test-unit-coverage` produces a readable `coverage-summary.json` on the default CI path
- `Post builder-web coverage summary` shows numeric totals for at least one representative successful run
- any remaining known failure classes are documented with a short troubleshooting note

#### Links
- Coverage workflow: `./.github/workflows/post-builder-web-coverage-summary.yml`
- Summary helper: `./support/ci/write_builder_web_coverage_summary.sh`
- README coverage notes: `./components/builder-web/README.md`

### 2. Expand builder-api contract boundaries

Protect one more stable JSON boundary with a deterministic golden or schema-style contract test.

#### Acceptance
- add at least one additional deterministic contract test around a stable builder-api JSON boundary
- keep the test isolated from database setup, or document why isolation is not feasible
- wire the new contract into the existing focused workflow or add a similarly scoped CI path

#### Links
- Contract workflow: `./.github/workflows/validate-builder-api-contracts.yml`
- Existing contract helper: `./components/builder-api/src/server/helpers.rs`
- Existing fixture: `./components/builder-api/tests/fixtures/package_results_contract.json`

### 3. Harden lightweight shell scanning

Keep shell scanning useful enough to stay enabled by tightening scope, runtime, and review output.

#### Acceptance
- shell scanning excludes generated or vendored content consistently across local and CI runs
- workflow runtime and output are reviewed, and any noisy exclusions are documented with justification
- the workflow emits enough evidence to quickly identify the first failing script when ShellCheck finds an issue

#### Links
- Shell scan workflow: `./.github/workflows/validate-shell-scripts.yml`
- Scan script: `./test/shellcheck.sh`
- Make target: `./Makefile`

### 4. Standardize focused CI evidence summaries

Make focused workflows publish a more consistent summary shape across coverage, contracts, and lightweight scans.

#### Acceptance
- at least two focused workflows publish a common summary structure such as status, evidence, and next-step hints
- failure paths still write useful evidence to the job summary instead of only raw logs
- workflows remain non-blocking unless explicitly promoted later

#### Links
- Coverage summary workflow: `./.github/workflows/post-builder-web-coverage-summary.yml`
- Shell scan workflow: `./.github/workflows/validate-shell-scripts.yml`
- Contract workflow: `./.github/workflows/validate-builder-api-contracts.yml`
