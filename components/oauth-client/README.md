# oauth-client

Shared OAuth provider client code used by Builder authentication flows.

This crate also carries the shared timeout/backoff helper for provider HTTP
requests so retry behavior stays consistent where it is enabled.

## Security hygiene rules

This component exchanges authorization codes for access tokens and then fetches
userinfo payloads from upstream providers. Those responses can contain bearer
tokens, account identifiers, and email addresses, so logging and request
construction must stay conservative.

Current guardrails:

- `OAuth2Cfg` debug output redacts `client_secret`.
- Provider debug logs record only the HTTP status and redacted body length for
  token and userinfo responses.
- Provider operations emit consistent counters for authenticate, token, and
  userinfo attempts plus failure counters for HTTP 4xx, HTTP 5xx/other,
  transport, and parse errors.
- Token exchange requests use encoded form bodies instead of hand-built query
  strings, so `client_secret`, `code`, and `redirect_uri` are not embedded in
  URLs and opaque codes are percent-encoded correctly.

## Provider notes

- GitHub token exchange intentionally sends `client_id`, `client_secret`, and
  `code`.
- GitLab, Okta, Azure AD, Active Directory, and Chef Automate also send
  `grant_type=authorization_code` and `redirect_uri`.
- Bitbucket keeps client credentials in HTTP basic auth and uses the form body
  only for the grant parameters.

## Residual risk

`oauth-client::Error::HttpResponse` still carries the upstream body so
`builder-api` can preserve current unauthorized response behavior. The error's
display text is redacted for logs, but callers that intentionally forward the
raw body should treat it as secret-bearing data.

## Validation

```bash
cargo test -p oauth-client
```

Timeout/backoff defaults:

- `request_timeout_ms = 3000`
- `request_retry_count = 2`
- `request_backoff_base_ms = 250`

The helper retries only transport-layer request errors. HTTP status failures and
JSON parse failures remain single-attempt results so invalid OAuth exchanges are
not repeated.

For live metric validation, point `HAB_STATS_ADDR` at a local UDP listener and
exercise a login flow that uses the provider you want to inspect:

```bash
nc -u -l 8125
```

Then, in another shell, run the Builder flow with:

```bash
HAB_STATS_ADDR=127.0.0.1:8125 <builder auth flow>
```

Expected packet names follow the provider key from `OAuth2Cfg.provider`, for
example:

- `bldr.github.authenticate:1|c`
- `bldr.github.token:1|c`
- `bldr.github.userinfo:1|c`
- `bldr.github.token.failure.http-4xx:1|c`
- `bldr.github.userinfo.failure.transport:1|c`

## Rollback

If a provider rejects the encoded form submission unexpectedly:

1. Revert the affected provider file(s) in `src/`.
2. Re-run `cargo test -p oauth-client`.
3. Re-test the matching Builder login flow before restoring any broader logging.

If timeout/backoff tuning causes unwanted latency:

1. Reset the provider config to the default timeout/backoff fields.
2. Re-run `cargo test -p oauth-client`.
3. Confirm retry-attempt log lines disappear during the next healthy login flow.

## Ops runbook: timeout and backoff

Symptoms:

- intermittent connect or TLS failures to an upstream OAuth provider
- slow upstream responses that usually succeed on the next attempt
- short-lived provider edge instability during token or userinfo requests

Tuning guidance:

1. Raise `request_timeout_ms` if the provider is slow but healthy.
2. Raise `request_retry_count` only for transient transport failures; do not use
   it to mask persistent 4xx or 5xx OAuth responses.
3. Raise `request_backoff_base_ms` if retries need more spacing to reduce load
   against an unstable provider.

Worst-case latency for a helper-managed request is approximately:

```text
(request_retry_count + 1) * request_timeout_ms + backoff_delays
```

Validation after tuning:

1. Run `cargo test -p oauth-client`.
2. Exercise a staging login flow for the provider you changed.
3. Inspect logs for `retry attempt=` entries to confirm whether failures are
   transport retries or one-shot OAuth response failures.

## Patch provenance

This was applied as a scripted in-repo patch set. No open code-scanning alert
was available to autofix in GitHub at the time of the update.
