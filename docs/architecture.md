# Builder repository architecture

This diagram is generated from a checked-in architecture model. Solid arrows show request or data movement. Dashed arrows show shared code or configuration dependencies that shape how runtime pieces fit together.

## Change summary since the last documented snapshot

- Automation: this document is now generated from `support/ci/architecture-model.json` by `support/ci/generate_architecture_doc.py` and freshness-checked in CI.
- Added component coverage: `components/builder-memcached/`.
- Added flow coverage: `components/builder-api/` -> `components/builder-memcached/` (Flow 3: hot package/cache lookups).

## Diagram

```mermaid
flowchart LR
    subgraph entrypoints["Entry points"]
        proxy["components/builder-api-proxy/<br/>Proxy + static asset host"]
        web["components/builder-web/<br/>Angular SPA"]
    end

    subgraph services["Services and integrations"]
        artifactory["components/artifactory-client/<br/>External package publish client"]
        api["components/builder-api/<br/>HTTP API gateway"]
        github["components/github-api-client/<br/>GitHub API client"]
        oauth["components/oauth-client/<br/>OAuth exchange client"]
    end

    subgraph data["Data and state"]
        db["components/builder-db/<br/>PostgreSQL access layer"]
        memcached["components/builder-memcached/<br/>Cache configuration and runtime cache path"]
        minio["components/builder-minio/<br/>S3-compatible artifact store"]
    end

    subgraph shared["Shared crates and config"]
        core["components/builder-core/<br/>Shared service logic"]
        datastore["components/builder-datastore/<br/>Datastore configuration"]
        protocol["components/builder-protocol/<br/>Shared message contracts"]
    end

    web -->|"Flow 1: browser requests"| proxy
    proxy -->|"Flow 1: API/UI routing"| api
    api -->|"Flow 2: sign-in and identity exchange"| oauth
    api -->|"Flow 2: GitHub-backed repo and org lookups"| github
    api -->|"Flow 3: package metadata"| db
    api -->|"Flow 3: package blobs"| minio
    api -->|"Flow 3: hot package/cache lookups"| memcached
    api -->|"Flow 3: external package publishing"| artifactory
    api -.->|"shared helpers"| core
    api -.->|"shared contracts"| protocol
    db -.->|"configured by"| datastore
    proxy -.->|"serves built UI assets from"| web
```

## Data flows

1. **UI request flow:** `components/builder-web/` drives browser requests through `components/builder-api-proxy/`, which fronts the HTTP surface exposed by `components/builder-api/`.
2. **Authentication and GitHub flow:** `components/builder-api/` uses `components/oauth-client/` for OAuth exchanges and `components/github-api-client/` for GitHub-backed operations such as repository or organization lookups.
3. **Package storage and publishing flow:** `components/builder-api/` coordinates package metadata through `components/builder-db/`, stores package blobs through `components/builder-minio/`, uses `components/builder-memcached/` for cache-backed hot paths, and can publish outward through `components/artifactory-client/`.

## Notes

- `components/builder-core/` and `components/builder-protocol/` are shared crates used by `components/builder-api/`, so they appear as supporting dependencies rather than independent network hops.
- `components/builder-datastore/` is shown as the configuration layer that shapes how `components/builder-db/` is wired, rather than as a separately exposed user-facing entry point.
- `components/builder-memcached/` is represented as a cache-facing dependency near the API/data plane so the diagram reflects cache-backed lookup paths that were absent from the prior snapshot.
