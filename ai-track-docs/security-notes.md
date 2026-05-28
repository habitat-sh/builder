# Security Notes

## Secret hygiene

- Keep real credentials in local-only files such as `.secrets/habitat-env` or user-home files like `~/.tokens`; do not commit them.
- Samples and documentation should use placeholders such as `<set-locally>` instead of live-looking secrets.
- Restrict local secret files to the current user where possible, for example `chmod 600 ~/.tokens`.

## Ignore rules

The repository `.gitignore` now covers common secret-bearing file types:

- `.env` and `.env.*`
- `*.pem`
- `*.p12`, `*.pfx`, `*.pkcs12`
- `*.jks`, `*.keystore`
- `*.secret`, `*.secrets`

These rules reduce the chance of accidentally staging ad-hoc local credentials while preserving the existing tracked sample file under `.secrets/habitat-env.sample`.

## Remediated risk

An obvious risk was present in `.secrets/habitat-env.sample`, which contained a concrete `OAUTH_CLIENT_SECRET` value. That sample now uses the placeholder `<set-locally>` so contributors are prompted to inject a local secret instead of reusing or propagating a checked-in value.
