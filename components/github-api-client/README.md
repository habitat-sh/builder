# github-api-client resilience tuning

`github-api-client` now has a small resilience helper for **idempotent GET requests**. It is
currently wired into:

- `GitHubClient::meta`
- `GitHubClient::repo`

The helper retries transient transport failures and retryable response statuses (`408`, `429`,
`500`, `502`, `503`, `504`). For `429 Too Many Requests`, it prefers GitHub's `Retry-After` or
`X-RateLimit-Reset` headers before falling back to the configured backoff.

## Tuning knobs

These settings live under the existing `[github]` config:

```toml
[github]
request_timeout_ms = 2000
retry_backoff_ms = 250
retry_attempts = 2
```

- `request_timeout_ms`: per-attempt timeout for retryable GET requests
- `retry_backoff_ms`: fixed delay between retry attempts when GitHub does not provide a retry hint
- `retry_attempts`: number of retries after the first attempt

## Scope and safety

- This helper is intentionally limited to **GET** paths.
- It should not be used for `POST`, `PUT`, or `DELETE` flows such as installation-token creation.
- Keep timeout and retry values conservative so callers fail fast instead of tying up request
  workers behind a slow upstream dependency.
