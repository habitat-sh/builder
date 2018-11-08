CREATE SEQUENCE IF NOT EXISTS origin_secrets_id_seq;
CREATE SEQUENCE IF NOT EXISTS origin_package_id_seq;
CREATE SEQUENCE IF NOT EXISTS origin_channel_id_seq;
CREATE SEQUENCE IF NOT EXISTS origin_integration_id_seq;
CREATE SEQUENCE IF NOT EXISTS origin_invitations_id_seq;
CREATE SEQUENCE IF NOT EXISTS origin_private_encryption_key_id_seq;
CREATE SEQUENCE IF NOT EXISTS origin_project_integration_id_seq;
CREATE SEQUENCE IF NOT EXISTS origin_project_id_seq;
CREATE SEQUENCE IF NOT EXISTS origin_public_key_id_seq;
CREATE SEQUENCE IF NOT EXISTS origin_secret_key_id_seq;
CREATE SEQUENCE IF NOT EXISTS origin_id_seq;
CREATE SEQUENCE IF NOT EXISTS origin_public_encryption_key_id_seq;

CREATE TABLE IF NOT EXISTS origins (
    id bigint DEFAULT next_id_v1('origin_id_seq') PRIMARY KEY NOT NULL,
    name text UNIQUE,
    owner_id bigint,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    default_package_visibility text DEFAULT 'public'::text NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_package (
    origin_id bigint,
    package_id bigint,
    channel_id bigint,
    operation smallint,
    trigger smallint,
    requester_id bigint,
    requester_name text,
    created_at timestamp with time zone DEFAULT now()
);

CREATE TABLE IF NOT EXISTS audit_package_group (
    origin_id bigint,
    channel_id bigint,
    package_ids bigint[],
    operation smallint,
    trigger smallint,
    requester_id bigint,
    requester_name text,
    group_id bigint,
    created_at timestamp with time zone DEFAULT now()
);

CREATE TABLE IF NOT EXISTS origin_secrets (
    id bigint DEFAULT next_id_v1('origin_secrets_id_seq') PRIMARY KEY NOT NULL,
    origin_id bigint REFERENCES origins(id),
    owner_id bigint,
    name text,
    value text,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    UNIQUE (origin_id, name)
);

CREATE TABLE IF NOT EXISTS origin_packages (
    id bigint DEFAULT next_id_v1('origin_package_id_seq') PRIMARY KEY NOT NULL,
    origin_id bigint REFERENCES origins(id),
    owner_id bigint,
    name text,
    ident text UNIQUE,
    ident_array text[],
    checksum text,
    manifest text,
    config text,
    target text,
    deps text,
    tdeps text,
    exposes text,
    scheduler_sync boolean DEFAULT false,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    visibility text DEFAULT 'public'::text NOT NULL
);

CREATE TABLE IF NOT EXISTS origin_channels (
    id bigint DEFAULT next_id_v1('origin_channel_id_seq') PRIMARY KEY NOT NULL,
    origin_id bigint REFERENCES origins(id),
    owner_id bigint,
    name text,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    UNIQUE (origin_id, name)
);

CREATE TABLE IF NOT EXISTS origin_integrations (
    id bigint DEFAULT next_id_v1('origin_integration_id_seq') PRIMARY KEY NOT NULL,
    origin text,
    integration text,
    name text,
    body text,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    UNIQUE (origin, integration, name)
);

CREATE TABLE IF NOT EXISTS origin_invitations (
    id bigint DEFAULT next_id_v1('origin_invitations_id_seq') PRIMARY KEY NOT NULL,
    origin_id bigint REFERENCES origins(id),
    origin_name text,
    account_id bigint,
    account_name text,
    owner_id bigint,
    ignored boolean DEFAULT false,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    UNIQUE (origin_id, account_id)
);

CREATE TABLE IF NOT EXISTS origin_private_encryption_keys (
    id bigint DEFAULT next_id_v1('origin_private_encryption_key_id_seq') PRIMARY KEY NOT NULL,
    origin_id bigint REFERENCES origins(id),
    owner_id bigint,
    name text,
    revision text,
    full_name text UNIQUE,
    body bytea,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now()
);

CREATE TABLE IF NOT EXISTS origin_projects (
    id bigint DEFAULT next_id_v1('origin_project_id_seq') PRIMARY KEY NOT NULL,
    origin_id bigint REFERENCES origins(id),
    origin_name text,
    package_name text,
    name text,
    plan_path text,
    owner_id bigint,
    vcs_type text,
    vcs_data text,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    vcs_auth_token text,
    vcs_username text,
    vcs_installation_id bigint,
    visibility text DEFAULT 'public'::text NOT NULL,
    auto_build boolean DEFAULT true NOT NULL,
    UNIQUE (origin_name, package_name, name)
);

CREATE TABLE IF NOT EXISTS origin_project_integrations (
    id bigint DEFAULT next_id_v1('origin_project_integration_id_seq') PRIMARY KEY NOT NULL,
    origin text NOT NULL,
    body text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    project_id bigint NOT NULL REFERENCES origin_projects(id) ON DELETE CASCADE,
    integration_id bigint NOT NULL REFERENCES origin_integrations(id) ON DELETE CASCADE,
    UNIQUE (project_id, integration_id)
);

CREATE TABLE IF NOT EXISTS origin_public_encryption_keys (
    id bigint DEFAULT next_id_v1('origin_public_key_id_seq') PRIMARY KEY NOT NULL,
    origin_id bigint REFERENCES origins(id),
    owner_id bigint,
    name text,
    revision text,
    full_name text UNIQUE,
    body bytea,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now()
);

CREATE TABLE IF NOT EXISTS origin_public_keys (
    id bigint DEFAULT next_id_v1('origin_public_key_id_seq') PRIMARY KEY NOT NULL,
    origin_id bigint REFERENCES origins(id),
    owner_id bigint,
    name text,
    revision text,
    full_name text UNIQUE,
    body bytea,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now()
);

CREATE TABLE IF NOT EXISTS origin_secret_keys (
    id bigint DEFAULT next_id_v1('origin_secret_key_id_seq') PRIMARY KEY NOT NULL,
    origin_id bigint REFERENCES origins(id),
    owner_id bigint,
    name text,
    revision text,
    full_name text UNIQUE,
    body bytea,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now()
);

CREATE TABLE IF NOT EXISTS origin_channel_packages (
    channel_id bigint NOT NULL REFERENCES origin_channels(id) ON DELETE CASCADE,
    package_id bigint NOT NULL REFERENCES origin_packages(id),
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    PRIMARY KEY (channel_id, package_id)
);

CREATE TABLE IF NOT EXISTS origin_members (
    origin_id bigint NOT NULL REFERENCES origins(id),
    origin_name text,
    account_id bigint NOT NULL,
    account_name text,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    PRIMARY KEY (origin_id, account_id)
);

-- USED IN JOBSRV STILL
CREATE OR REPLACE FUNCTION get_all_origin_packages_for_ident_v1(op_ident text) RETURNS SETOF origin_packages
    LANGUAGE plpgsql STABLE
    AS $$
  BEGIN
    RETURN QUERY SELECT * FROM origin_packages WHERE ident LIKE (op_ident || '%') ORDER BY ident;
    RETURN;
  END
  $$;

CREATE INDEX IF NOT EXISTS origin_packages_ident_array ON origin_packages(ident_array);
