-- Creating the audit table again, because calling it "audit" was short-sighted
CREATE TABLE IF NOT EXISTS audit_package (
  origin_id bigint,
  package_id bigint,
  channel_id bigint,
  operation smallint,
  trigger smallint,
  requester_id bigint,
  requester_name text,
  created_at timestamptz DEFAULT now()
);

-- This is our new table
CREATE TABLE IF NOT EXISTS audit_package_group (
  origin_id bigint,
  channel_id bigint,
  package_ids bigint[],
  operation smallint,
  trigger smallint,
  requester_id bigint,
  requester_name text,
  group_id bigint,
  created_at timestamptz DEFAULT now()
);

CREATE OR REPLACE FUNCTION add_audit_package_entry_v1 (
  p_origin_id bigint,
  p_package_id bigint,
  p_channel_id bigint,
  p_operation smallint,
  p_trigger smallint,
  p_requester_id bigint,
  p_requester_name text
) RETURNS SETOF audit_package AS $$
INSERT INTO audit_package (origin_id, package_id, channel_id, operation, trigger, requester_id, requester_name)
VALUES (p_origin_id, p_package_id, p_channel_id, p_operation, p_trigger, p_requester_id, p_requester_name)
RETURNING *;
$$ LANGUAGE SQL VOLATILE;

CREATE OR REPLACE FUNCTION add_audit_package_group_entry_v1 (
  p_origin_id bigint,
  p_channel_id bigint,
  p_package_ids bigint[],
  p_operation smallint,
  p_trigger smallint,
  p_requester_id bigint,
  p_requester_name text,
  p_group_id bigint
) RETURNS SETOF audit_package_group AS $$
INSERT INTO audit_package_group (origin_id, channel_id, package_ids, operation, trigger, requester_id, requester_name, group_id)
VALUES (p_origin_id, p_channel_id, p_package_ids, p_operation, p_trigger, p_requester_id, p_requester_name, p_group_id)
RETURNING *;
$$ LANGUAGE SQL VOLATILE;
