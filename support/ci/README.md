# support/ci shell strictness

`support/ci/*.sh` is gated by a stricter ShellCheck pass:

```sh
make scan-shell-ci-strict
```

That target runs:

```sh
shellcheck -x -P SCRIPTDIR support/ci/*.sh
```

The stricter gate is intentionally limited to `support/ci` so we can follow sourced files and
raise shell quality on CI-critical scripts without turning the full repository into a single
cleanup project.

## Documented suppressions

The strict gate still allows a few inline suppressions in `support/ci` where the behavior is
intentional:

| File | Code | Reason |
| --- | --- | --- |
| `support/ci/shared.sh` | `SC1091` | `"$HOME/.cargo/env"` is created by rustup at runtime, so it cannot be resolved statically in every environment. |
| `support/ci/rustfmt.sh` | `SC1094` | the repo-root `source ./support/ci/shared.sh` path is kept for Buildkite compatibility. |
| `support/ci/builder-base-plan.sh` | `SC2154` | Habitat plan variables such as `pkg_prefix`, `pkg_release`, and `pkg_target` are injected by the build environment. |
| `support/ci/fast_pass.sh` | `SC2086` | word splitting is deliberate because the script iterates whitespace-delimited file and directory lists from CI environment variables. |

New suppressions in this path should stay inline, explain why the warning is safe to ignore, and
prefer the narrowest possible scope.
