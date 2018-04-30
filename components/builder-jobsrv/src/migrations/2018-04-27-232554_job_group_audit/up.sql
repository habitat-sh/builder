CREATE TABLE IF NOT EXISTS audit (
    group_id bigint,
    operation smallint,
    trigger smallint,
    requester_id bigint,
    requester_name text,
    created_at timestamptz DEFAULT now()
);

CREATE OR REPLACE FUNCTION add_audit_entry_v1 (
  p_group_id bigint,
  p_operation smallint,
  p_trigger smallint,
  p_requester_id bigint,
  p_requester_name text
) RETURNS SETOF audit AS $$
      INSERT INTO audit (group_id, operation, trigger, requester_id, requester_name)
      VALUES (p_group_id, p_operation, p_trigger, p_requester_id, p_requester_name)
      RETURNING *;
$$ LANGUAGE SQL VOLATILE;
