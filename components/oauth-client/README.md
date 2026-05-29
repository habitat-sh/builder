# oauth-client

Shared OAuth provider client code used by Builder authentication flows.

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

## Patch provenance

This was applied as a scripted in-repo patch set. No open code-scanning alert
was available to autofix in GitHub at the time of the update.
