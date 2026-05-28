# Extending Token Generator

`tools/token-generator` is intentionally small: parse CLI args, initialize logging, validate the signing key path, generate a token with `builder_core::access_token::AccessToken`, and print the result.

See also [dependency-notes.md](dependency-notes.md) for the module's critical dependency summary and low-risk pinning recommendations.

## Safe extension points

- **Add new CLI flags** in `src/main.rs` on the `Args` struct using Clap attributes.
- **Adjust logging behavior** through `log_level()` or `init_logging()`.
- **Add preflight validation** before `AccessToken::user_token(...)` if new inputs require explicit checks. Prefer mapping raw filesystem failures to explicit user-facing errors at this boundary.

## Keep behavior predictable

- Keep token generation delegated to `AccessToken::user_token(...)` so this tool stays aligned with Builder's token format.
- Prefer small pure helpers when adding logic; they are easy to test without filesystem or key material dependencies.
- Continue printing only the generated token to stdout so scripts can consume it directly.

## Minimal test guidance

- Add unit tests next to the code in `src/main.rs`.
- Prefer deterministic tests around argument parsing and helper functions.
- Avoid tests that require real signing keys unless the change specifically targets integration behavior.
