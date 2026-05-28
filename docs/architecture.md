# Builder repository architecture

This diagram maps Builder's main runtime and support components to their real paths in this
repository. Solid arrows show request or data movement. Dashed arrows show shared code or config
dependencies that shape how the runtime pieces fit together.

```mermaid
flowchart LR
    subgraph entrypoints["Entry points"]
        web["components/builder-web/<br/>Angular SPA"]
        proxy["components/builder-api-proxy/<br/>Proxy + static asset host"]
    end

    subgraph services["Services and integrations"]
        api["components/builder-api/<br/>HTTP API gateway"]
        oauth["components/oauth-client/<br/>OAuth exchange client"]
        github["components/github-api-client/<br/>GitHub API client"]
        artifactory["components/artifactory-client/<br/>External package publish client"]
        db["components/builder-db/<br/>PostgreSQL access layer"]
        minio["components/builder-minio/<br/>S3-compatible artifact store"]
    end

    subgraph shared["Shared crates and config"]
        core["components/builder-core/<br/>Shared service logic"]
        protocol["components/builder-protocol/<br/>Shared message contracts"]
        datastore["components/builder-datastore/<br/>Datastore configuration"]
    end

    web -->|"Flow 1: browser requests"| proxy
    proxy -->|"Flow 1: API/UI routing"| api

    api -->|"Flow 2: sign-in and identity exchange"| oauth
    api -->|"Flow 2: GitHub-backed repo and org lookups"| github

    api -->|"Flow 3: package metadata"| db
    api -->|"Flow 3: package blobs"| minio
    api -->|"Flow 3: external package publishing"| artifactory

    api -.->|"shared helpers"| core
    api -.->|"shared contracts"| protocol
    db -.->|"configured by"| datastore
    proxy -.->|"serves built UI assets from"| web
```

## Data flows

1. **UI request flow:** `components/builder-web/` drives browser requests through
   `components/builder-api-proxy/`, which fronts the HTTP surface exposed by
   `components/builder-api/`.
2. **Authentication and GitHub flow:** `components/builder-api/` uses `components/oauth-client/`
   for OAuth exchanges and `components/github-api-client/` for GitHub-backed operations such as
   repository or organization lookups.
3. **Package storage and publishing flow:** `components/builder-api/` coordinates package metadata
   through `components/builder-db/`, stores package blobs through the S3-compatible path represented
   by `components/builder-minio/`, and can publish outward through
   `components/artifactory-client/`.

## Notes

- `components/builder-core/` and `components/builder-protocol/` are shared crates used by
  `components/builder-api/`, so they appear as supporting dependencies rather than independent
  network hops.
- `components/builder-datastore/` is shown as the configuration layer that shapes how
  `components/builder-db/` is wired, rather than as a separately exposed user-facing entry point.
