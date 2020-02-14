CREATE SEQUENCE IF NOT EXISTS audit_origin_id_seq;
CREATE TYPE origin_operation AS ENUM ('origin_create', 'origin_delete', 'owner_transfer');

CREATE TABLE IF NOT EXISTS audit_origin (
    id bigint DEFAULT next_id_v1('audit_origin_id_seq') PRIMARY KEY NOT NULL,
    operation origin_operation,
    target_object text,
    origin text,
    requester_id bigint,
    requester_name text,
    created_at timestamp with time zone DEFAULT now()
);
