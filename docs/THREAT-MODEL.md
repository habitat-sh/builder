# Threat Model: `components/builder-api` (STRIDE-A)

Scope: the actix-web REST service under `/v1`, plus the code it depends on directly: `builder-core` (tokens, keys, crypto), `builder-db` (models and queries), and `oauth-client`. Based on a static code review; no dynamic testing was performed. Section 2 covers the API-wide findings; sections 2.1-2.9 cover individual files and components in more depth. "A" is interpreted as **Abuse** (resource and business-logic abuse). The "P-11" reference was not defined in the repository, so this document has not been mapped to it.

## 1. System overview

- OAuth login, origins and RBAC, package upload/download, channels, secrets and keys, access tokens, events.
- Authentication: a global optional-auth middleware (`framework/middleware.rs`) accepts `Authorization: Bearer`. It validates the token signature with the Builder key, checks it against the account's single DB token, and caches the session in Memcached.
- Authorization: per handler via `authorize_session(origin, min_role)` (`server/authorize.rs`).
- Dependencies: PostgreSQL, Memcached, S3/MinIO or Artifactory, OAuth provider, license server, SaaS Builder (`events/saas`).

### Trust boundaries

| Boundary | Crossing |
|---|---|
| TB1 | Internet / `hab` client to API (TLS and mTLS optional; default bind `0.0.0.0`, no TLS) |
| TB2 | API to Postgres, Memcached, S3/Artifactory |
| TB3 | API to OAuth provider, license server, SaaS Builder |
| TB4 | Build workers to API (`BUILD_WORKER` flag) |
| TB5 | Uploaded `.hart` content to API parsing |

### Assets

Origin secret signing keys, origin encryption keys, origin secrets, access and OAuth tokens, package artifacts and their integrity, membership and role data, Builder key material, service availability.

## 2. STRIDE-A analysis

| Cat | Threat | Evidence | Severity | Recommendation |
|---|---|---|---|---|
| S | `HAB_FUNC_TEST` env var makes `authenticate` and the middleware mint sessions for fixed tokens (`bobo`, `wesker`, ...). | `middleware.rs::handle_test_environment`, `authenticate.rs::do_authenticate` | High if set in production | Gate behind `cfg(test)` or a feature flag; refuse to start if set alongside production config. |
| S | Token comparison uses `!=` after trimming `=`. | `validate_token_count_and_match` | Low | Use constant-time comparison. |
| S | Tokens have no expiry or scoping; one token per account. A leaked token grants full account access. | `profile.rs` | Medium | Add expiry and scopes; audit token use. |
| T | Upload does not verify the artifact signature against the origin public key; only a client-supplied checksum and ident are checked. | `do_upload_package_finish` | Medium-High | Verify signatures before accepting. |
| T | `forced=true` bypasses the existing-package conflict check and re-uploads to the store (checksum-equality guard applies). | `do_upload_package_start/finish` | Low-Medium | Restrict and log forced uploads. |
| T | Temp files may remain on some failure paths; artifact is renamed into `data_path` before all checks finish. | `do_upload_package_*` | Low | Use a cleanup guard; verify before rename. |
| T | Unencoded interpolation into upstream URLs: `query`/`channel` in `events/saas`, `licenseId` in the license check. Enables parameter injection. | `events.rs`, `helpers.rs::fetch_license_expiration` | Medium | Build URLs with `Url` / `.query()` encoding. |
| R | Sensitive actions (role changes, key/secret ops, token create/revoke, forced uploads) are logged only at `debug!`; no structured audit trail. | `origins.rs`, `profile.rs` | Medium | Emit info-level audit events with actor, origin, action, source IP. |
| R | Access log lacks user and request IDs. | `server/mod.rs` | Low | Add session and request IDs. |
| I | Secrets in logs: OAuth code (`authenticate called, code = ...`), session with OAuth token (`issuing session, {:?}`), `trace!` token logging. | `authenticate.rs`, `middleware.rs` | High where debug/trace enabled | Never log tokens or codes; redact `Session` `Debug`. |
| I | Verbose errors returned to clients (`err={:?}`, `License API error: {}`). | `pkgs.rs`, `helpers.rs` | Low-Medium | Return generic messages; log details server-side. |
| I | Any origin `Member` can download the latest secret signing key. | `origins.rs::download_latest_origin_secret_key` | Medium | Confirm intent; otherwise require Administrator. |
| I | Cached role/membership can be stale, leaking private packages after removal. The `member:` boolean cache is never cleared (see 2.8). | `authorize.rs`, memcache | Medium | Invalidate on change; short TTL. |
| I | Default S3 secret `"password"`; sample config contains hard-coded secrets. | `config.rs` | Medium | Fail startup on default credentials. |
| I | `events/saas` forwards the caller's `Authorization` header to `saas_bldr_url`. | `events.rs` | Low (config-controlled) | Keep URL trusted/https; avoid forwarding unless needed. |
| D | Uploads stream to disk with no size cap, quota, or rate limit. | `do_upload_package_async` | High | Enforce size limits and per-origin quotas. |
| D | No rate limiting on `/authenticate/{code}`, token generation, search, downloads. | `server/mod.rs` | Medium | Add rate limiting (proxy or middleware). |
| D | `hdr.to_str().unwrap()` panics on non-ASCII `Authorization`; other request-reachable `unwrap()`s (`events.rs` header copy, `PackageTarget::from_str`). | `middleware.rs:41` | Medium | Return 400/401; remove `unwrap()`s. |
| D | Blocking calls in async handlers (`reqwest::blocking` license check; blocking DB/Memcached) can exhaust workers. | `helpers.rs`, `profile.rs` | Medium | Use async with timeouts. |
| D | `AppState::new` per worker/request may stress connection pools. | `server/mod.rs` | Low-Medium | Verify pooling. |
| E | `BUILD_WORKER` flag returns early from `authorize_session` with no origin check; the Builder account is Owner of every origin. | `authorize.rs` | High if worker token stolen | Scope worker tokens; rotate; restrict routes. |
| E | `FeatureFlags::from_bits(...).unwrap()` panics on unknown bits. | `authorize.rs` | Low | Use `from_bits_truncate` or handle error. |
| E | `authorize_session(...).unwrap()` re-check after upload. | `pkgs.rs:1251` | Low | Pass session through. |
| E | Role escalation controls (`update_origin_member_role`, transfer, member delete) look correct. | `origins.rs` | n/a | Keep regression tests. |
| A | Origin name squatting; any account can claim any valid name. | `origins.rs::create_origin` | Medium | Reserved names, per-user limits, review. |
| A | Free hosting of malicious packages; no scanning or quotas; `Hidden` packages downloadable. | `download_package` | Medium | Quotas, abuse reporting, takedown tooling. |
| A | Supply-chain abuse by promoting a malicious build to `stable` via a compromised Maintainer/Member. | `channels.rs` | High impact | Signature enforcement, promotion approvals, audit. |
| A | Invitation spam; user/origin enumeration through public routes. | `origins.rs` | Low-Medium | Rate-limit; uniform errors. |
| A | Memcached key injection/poisoning via user-controlled identifiers if not validated. | `services/memcache.rs` | Low-Medium | Validate/escape identifiers in keys. |
| A | License key brute force via `/profile/license` proxying. | `profile.rs::set_license` | Low-Medium | Per-account rate limit. |

### 2.1 Channels (`resources/channels.rs`)

| Cat | Threat | Evidence | Severity | Recommendation |
|---|---|---|---|---|
| S/E | Promote, demote, and channel create/delete require `Maintainer`. A compromised Maintainer or token can promote any package to `stable`, and the bulk path moves every package from the source channel. | `promote_channel_packages`, `promote_package` | High impact | Add promotion approvals or protected-channel policy; audit all promotes. |
| T | Channel names from the URL and the `channel` query param are not validated in the handlers. `create_channel` and the auto-create branch of promote/demote pass them straight to `Channel::create`. | `create_channel`, `do_promote_or_demote_channel_packages` | Medium | Validate names (charset, length) and reject reserved prefixes. |
| T | Demote of `stable` is permitted for Maintainers. Only the delete of `stable`/`unstable` is blocked. | `demote_*`, `delete_channel` | Medium | Treat `stable` demotion as privileged. |
| D/A | Promote and demote auto-create unbounded channels (including snapshot channels) with no per-origin cap. Bulk promote loads whole channel membership and closure computation (`check=true`) is unbounded. | `promote_channel_packages`, `create_snapshot_channel`, `channel_package_closure` | Medium | Cap channels per origin and closure size; add timeouts. |
| R | Channel mutations are not audit logged (only `debug!`). | `channels.rs` | Medium | Emit audit events. |
| I | Read routes are open to anonymous callers. Visibility filtering relies on `visibility_for_optional_session`; any mistake exposes private packages. | `do_get_channel_*` | Medium | Add tests for private-origin access via every channel route. |
| E | Mixed status codes (`UNAUTHORIZED` for authorization failures) hide the real reason, and `create/delete_channel` discard the underlying error. | `create_channel`, `delete_channel` | Low | Return 403 for authorization failures. |
| D | Request-reachable `unwrap()` calls (`ident.parse().unwrap()`, `serde_json::to_*().unwrap()`). | `channels.rs:1235,1241,1420` | Low-Medium | Replace with error handling. |

### 2.2 Package settings (`resources/settings.rs`)

| Cat | Threat | Evidence | Severity | Recommendation |
|---|---|---|---|---|
| I/T | A `Maintainer` can switch a package from private to public via `PUT /settings/{origin}/{name}`. This irreversibly exposes content and is a likely data-exfiltration path for a compromised account. | `update_origin_package_settings` | Medium-High | Require Administrator for making a package public; audit. |
| T | The package name in the path is not validated before `create`, and `create` does not verify that the name is a valid package name. | `create_origin_package_settings` | Low-Medium | Validate the name. |
| R | Visibility and settings changes are not audit logged. | `settings.rs` | Medium | Emit audit events. |
| E | `GET` requires only membership in the origin, so private settings are visible to read-only members. | `get_origin_package_settings` | Low | Confirm intent. |
| I | Delete preflight error text is returned to the client (409 body). It is sanitized and low risk. | `delete_origin_package_settings` | Low | None required. |

### 2.3 Reverse dependencies (`resources/jobs.rs`)

| Cat | Threat | Evidence | Severity | Recommendation |
|---|---|---|---|---|
| D | `db.get_conn().map_err(...).unwrap()` panics on pool exhaustion or DB outage, killing the worker on an **unauthenticated** route. | `get_rdeps` | High | Return 503 instead of unwrapping. |
| I | Filtering uses the dependent origin's *default* visibility, not each package's visibility, so a private package in a public-default origin can appear in results. The queried `{origin}/{name}` itself is not authorization checked, which leaks the existence of private packages. | `filtered_rdeps`, `get_rdeps` | Medium | Filter on per-package visibility and authorize the target package. |
| I | Unfiltered rdeps are logged at `debug!` (`BEFORE FILTERING`). The second log prints the unfiltered value too. | `get_rdeps` | Low | Remove or fix the logging. |
| D | The route is unauthenticated, performs a graph traversal, and has no rate limit or result cap. | `reverse_dependencies.rs` | Medium | Add caps and rate limits. |
| T | The default target `x86_64-linux` is used when none is given; the target is not validated against `config.api.targets`. | `get_rdeps` | Low | Validate the target. |

### 2.4 S3 / MinIO storage (`services/s3.rs`)

| Cat | Threat | Evidence | Severity | Recommendation |
|---|---|---|---|---|
| I | Static credentials come from config or `MINIO_SECRET_KEY`, falling back to `"password"`. There is no support for short-lived IAM credentials. Plain `http://` endpoints are accepted for MinIO. | `S3Cfg::default`, `S3Handler::new` | Medium | Fail on defaults; require TLS to the object store; prefer IAM roles. |
| T | Object integrity depends on the client-supplied checksum and the checksum recorded in the DB. Downloads are verified with a streaming checksum (`checksum_verified_stream`), but the checksum is only checked at the end, so a client has already received data when a mismatch aborts the response. | `pkgs.rs::checksum_verified_stream` | Low-Medium | Clients must verify (hab does so with signatures); keep the Content-Length omission. |
| T | The `do_upload_package_finish` flow overwrites the object key for `forced` uploads; there is no object versioning or write-once policy. | `s3.rs::upload` | Medium | Enable bucket versioning or Object Lock. |
| D | Panics on `unwrap()` in `new` (region), `upload` (`metadata()`, `to_str()`), `bucket_exists` (`name.unwrap()`), and multipart (`upload_id.unwrap()`). Non-UTF-8 paths or odd S3 responses crash the handler. | `s3.rs:77,120,180,181,385,412` | Medium | Return errors instead. |
| D | Multipart uploads that fail midway may leave orphaned parts. There is no visible abort or lifecycle handling. | `multipart_upload` | Low-Medium | Abort on error; add a lifecycle rule for incomplete uploads. |
| I | Object keys are derived from the package ident (`origin/name/version/release/target`). Idents are validated on upload, which prevents key traversal. | `s3_key` | Low | Keep validation. |
| E | All objects for all origins share one bucket. Privacy is enforced only in the API, so a leaked credential exposes every private package. | `S3Cfg` | Medium | Restrict credentials and network access; consider per-origin prefixes with scoped policies. |

### 2.5 Token validation (`builder-core`: `access_token.rs`, `crypto.rs`, `keys.rs`, `privilege.rs`)

How it works: a token is `_` + base64 of a `SignedBox` (Builder key encryption) over a protobuf holding `account_id`, `flags`, and `expires`. The API then checks the token string against the account's one DB token. The Builder account (id 0) is the exception and skips the DB check.

| Cat | Threat | Evidence | Severity | Recommendation |
|---|---|---|---|---|
| S/E | User tokens never expire (`Duration::MAX`) and can only be revoked. A leaked token stays valid indefinitely. | `AccessToken::user_token` | High | Add finite lifetimes and rotation. |
| S/E | Builder tokens (account id 0) carry **all** flags (`ADMIN`, `EARLY_ACCESS`, `BUILD_WORKER`) and bypass the DB revocation check in the middleware. They cannot be revoked before their 2-hour expiry except by rotating the Builder key. | `bldr_token`, `middleware.rs::handle_builder_account` | High | Track issued Builder tokens or shorten their lifetime; scope the flags to what workers need. |
| E | New user tokens copy the caller's session flags. Flag-bearing sessions can mint long-lived tokens that preserve those privileges. | `profile.rs::generate_access_token` | Medium | Do not carry `ADMIN`/`BUILD_WORKER` into user tokens. |
| T | Hard-coded expiry exception (`8_210_298_326_400`) bypasses the normal timestamp check. A forged token would need the Builder key, so impact is limited, but the special case is fragile. | `validate_access_token` | Low | Fix the root cause and remove the special case. |
| I | Token plaintext is stored in the DB (`account_tokens.token`) and is used for comparison; a DB read exposes every token. `trace!` logs whole tokens in `from_str` and `validate_access_token`. | `middleware.rs`, `access_token.rs` | High | Store only a hash; never log tokens. |
| I | The Builder key can come from the `BLDR_SECRET_KEY` env var, which is exposed to anyone who can read the process environment. All token security depends on this single symmetric-style key. | `keys.rs` | Medium-High | Use a secrets manager; support key rotation with overlap; restrict process inspection. |
| T | A single key encrypts tokens, secrets, and origin keys. Compromise breaks all of them. | `crypto.rs` | Medium | Separate keys per purpose. |
| D | Unauthenticated callers can force a decryption for every request with a `_`-prefixed token. Parsing is cheap and failures are rejected early, so this is low risk. | `AccessToken::from_str` | Low | Rate limit failed auth. |
| R | Failed token validation is only `trace!` logged; there is no counter or alert for repeated failures. | `middleware.rs` | Medium | Emit metrics and warnings. |
| E | `FeatureFlags::from_bits(...).unwrap()` in `authorize.rs` panics on unknown flags, and `ADMIN`/`EARLY_ACCESS` have no visible enforcement beyond `BUILD_WORKER` in the files reviewed. | `authorize.rs`, `privilege.rs` | Low | Handle unknown bits; verify how `ADMIN` is used. |

### 2.6 Reverse dependency queries (`resources/reverse_dependencies.rs`)

| Cat | Threat | Evidence | Severity | Recommendation |
|---|---|---|---|---|
| T | Both queries use `sql_query` with bound parameters (`$1`..`$5`); no SQL injection found. | `get_rdeps`, `get_rdeps_for_ident` | Low | Keep binding. |
| D | `get_rdeps` self-joins `origin_packages` with a lateral `tdeps @>` scan and runs the same subquery twice in a `UNION`. Results are uncapped and the route is unauthenticated, so popular packages make it an expensive-query DoS vector. | `get_rdeps` | Medium | Remove the duplicate branch, add a `LIMIT` and pagination, require auth or rate limit, add a statement timeout. |
| I | The query returns reverse dependencies across all visibilities and relies on the caller (`filtered_rdeps`) to filter. A new call site that forgets to filter leaks private package names. | `get_rdeps_for_ident` | Low-Medium | Push visibility filtering into the query. |
| I | `debug!` prints the bound query and full results. | `get_rdeps` | Low | Remove or downgrade. |

### 2.7 Database models and search (`builder-db`)

| Cat | Threat | Evidence | Severity | Recommendation |
|---|---|---|---|---|
| T | Raw SQL is limited to `sql_query` with bound parameters (advisory locks, head-package selection) and constant `sql::<_>()` snippets; no string-built SQL with user input was found outside tests. | `models/channel.rs`, `models/package.rs` | Low | Keep this pattern; add a lint or review rule against `format!` in SQL. |
| T/D | Search passes the user's text into `to_tsquery(format!("{}:*", query))`. `to_tsquery` parses operator syntax, so malformed input (`&`, `|`, `!`, `:`, unbalanced parentheses) raises a Postgres syntax error (HTTP 500) and complex queries are expensive. | `package.rs` (`search`, `search_distinct`), `channel.rs` (events) | Medium | Use `plainto_tsquery` or sanitize input; cap query length; return 400 on parse errors. |
| D | `Package::search` and events queries are reachable anonymously and use `count(DISTINCT ...)` plus paging, which is costly on large tables. | `package.rs`, `channel.rs` | Medium | Rate limit; cap query length; add statement timeouts. |
| I | The origin secret encryption key is stored encrypted (migrations `encrypt_secret_keys`, `migrate_to_encrypted`). Account tokens (`account_tokens.token`) and license keys (`license_keys.license_key`) are plaintext in the DB. | `models/account.rs`, `models/license_keys.rs` | Medium-High | Hash tokens; encrypt or hash license keys. |
| E | Role enforcement is in handlers only; DB constraints do not enforce who may write which rows. A missed `authorize_session` call is a direct privilege escalation. | `resources/*.rs` | Medium | Add handler-level tests for every route and role. |

### 2.8 Memcached (`services/memcache.rs`)

| Cat | Threat | Evidence | Severity | Recommendation |
|---|---|---|---|---|
| I/E | Stale authorization: `check_origin_member` caches a boolean under `member:{origin}/{id}` (`set_origin_member`) but **nothing ever clears it**. Only the role key is cleared on role change and member removal (`clear_cache_for_member_role`). A removed member can keep `is_member` access for up to the cache TTL on code paths that use it. | `authorize.rs:119`, `memcache.rs:187`, `origins.rs:1231,1420` | Medium | Add a membership clear and call it on invite acceptance, removal, departure, and transfer. Use a short TTL. |
| T | Cache keys such as `member:{origin}/{id}`, `package:{origin}/{name}` and `channel:{origin}/{channel}` embed client-supplied strings without validation or hashing. A crafted origin name could collide with or poison other keys, or exceed the 250-byte memcached key limit. Session keys are SHA-512 hashed and are fine. | `memcache.rs` key helpers | Low-Medium | Validate identifiers before use in keys; hash long or untrusted components. |
| D | `MemcacheClient::new` does `connect(...).unwrap()` and `AppState::new` runs per worker, so a Memcached outage panics worker startup. `get_session` does `parse_from_bytes(...).unwrap()`, so a corrupted or poisoned cache entry panics the request. | `memcache.rs:36,129` | Medium | Return errors and treat cache failures as a miss. |
| T/E | Cache poisoning: Memcached is unauthenticated by default, and the session value in the cache **is** the authorization (id, name, flags). Anyone who can write to Memcached can mint a session with `BUILD_WORKER` or `ADMIN` flags for any token hash. TLS is optional. | `config.rs` `MemcacheCfg`, `middleware.rs` | High if the network is not isolated | Isolate the network, enable TLS and SASL auth, or sign cached sessions. |
| I | Package JSON is cached per account (`...:{account_id}`) which isolates private data; the `404` sentinel is cached as a string and could mask a later upload until cleared (`clear_cache_for_package` handles uploads). | `set_package`/`get_package` | Low | Keep tests for private versus public cache separation. |
| I | `trace!` logs the full token on `get_session`. | `memcache.rs` `get_session` | Medium | Log the hash, not the token. |

### 2.9 OAuth client (`components/oauth-client`)

| Cat | Threat | Evidence | Severity | Recommendation |
|---|---|---|---|---|
| S/E | **Account takeover by username reuse.** `session_create_oauth` calls `Account::find_or_create` keyed on `accounts.name` (the provider username) only. The provider's stable user id (`extern_id`) is put in the session token but never compared to the account. If a provider username is renamed and re-registered by someone else, or two configured providers share a username, the new person logs in as the existing account with all its origin memberships and tokens. | `middleware.rs::session_create_oauth`, `models/account.rs::find_or_create` | High | Persist and match on `(provider, extern_id)`; reject mismatches. |
| I | `debug!("Authenticate called, config: {:?}", self.config)` logs `OAuth2Cfg`, which derives `Debug` and contains `client_secret`. | `oauth-client/src/client.rs`, `config.rs` | High where debug logging is on | Implement a redacting `Debug`; do not log config. |
| I | `debug!("GitHub response body: {}", body)` logs the token-exchange response, including the user's `access_token`, plus user profile data. Other providers are likely similar. | `github.rs` (and sibling providers) | High where debug logging is on | Remove body logging. |
| I | The GitHub flow puts `client_id`, `client_secret` and `code` in the POST URL's query string, so they can land in proxy and server logs. | `github.rs::authenticate` | Medium | Send them in the form body. |
| S | The API's `/authenticate/{code}` accepts any code and has no `state` or PKCE check on the server side; login CSRF protection relies on the web UI. | `authenticate.rs` | Low-Medium | Enforce `state` or PKCE if the API is used directly. |
| I | OAuth provider error bodies are returned verbatim to the caller (`HttpResponse::Unauthorized().body(response)`). | `authenticate.rs` | Low | Return a generic message. |
| D | `OAuth2Client::new` panics on an unknown provider string, and the OAuth HTTP calls have no explicit timeout. | `client.rs` | Low-Medium | Return a config error; set timeouts. |
| E | The OAuth token is stored in the session and cached for 3 days (`SESSION_DURATION`); revoking the provider grant does not end the Builder session. | `middleware.rs` | Low-Medium | Shorten session lifetime or re-validate. |

## 3. Priorities

1. Upload size caps, quotas, and rate limits.
2. Fix account identity: match OAuth logins on `(provider, extern_id)`, not username alone.
3. Stop logging secrets (OAuth client secret and token responses, tokens, `trace!` in `builder-core` and `memcache.rs`); redact `Session` and `OAuth2Cfg` `Debug`.
4. Give user tokens a finite lifetime; store token hashes instead of plaintext.
5. Protect Memcached (network isolation, TLS, auth) because cached sessions grant privileges; clear the `member:` cache on membership changes.
6. Scope or contain `BUILD_WORKER` and the Builder super-account; make Builder tokens revocable or shorter-lived.
7. Verify package signatures on upload.
8. Remove request-reachable `unwrap()`s (notably the unauthenticated `get_rdeps` DB unwrap, the auth middleware header unwrap, Memcached `connect`/`get_session`, and the S3 unwraps); gate `HAB_FUNC_TEST`.
9. Protect `stable` and public-visibility changes (promotion approval, Administrator for making a package public).
10. Fix rdeps visibility filtering and cost; sanitize `to_tsquery` input; validate channel names.
11. Add an audit log; encode outbound URL parameters.
12. Invalidate role/membership caches on change.
