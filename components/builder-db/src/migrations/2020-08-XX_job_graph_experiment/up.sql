

CREATE TYPE target_arch AS ENUM (
    'x86_64-darwin', 'x86_64-linux', 'x86_64-linux-kernel2' ,'x86_64-windows'
);

CREATE TYPE job_exec_state AS ENUM (
  'Pending',
  'Dispatched',
  'Schedulable',
  'Eligible',
  'Built',
  'JobFailed',
  'DependencyFailed',
  'CancelPending',
  'CancelComplete'
);

CREATE SEQUENCE IF NOT EXISTS job_graph_id_seq;

CREATE TABLE IF NOT EXISTS job_graph (
    id BIGINT DEFAULT nextval('job_graph_id_seq') PRIMARY KEY NOT NULL,
    group_id BIGINT NOT NULL,
    job_state job_exec_state,
    plan_ident text,
    manifest_ident text,
    as_built_ident text,
    dependencies BIGINT[] NOT NULL,
    target_arch target_arch NOT NULL,
    -- may insert some more prioritzation stuff, around groups, etc.
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


