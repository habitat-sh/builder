ALTER TABLE audit_package_group ALTER COLUMN operation SET DATA TYPE origin_package_operation
    USING CASE operation
        WHEN 0 THEN 'promote'
        WHEN 1 THEN 'demote'
        ELSE NULL
    END :: origin_package_operation;

ALTER TABLE audit_package_group ALTER COLUMN trigger SET DATA TYPE package_channel_trigger
    USING CASE trigger
        WHEN 0 THEN 'unknown'
        WHEN 1 THEN 'builder_ui'
        WHEN 2 THEN 'hab_client'
        ELSE NULL
    END :: package_channel_trigger;

CREATE OR REPLACE FUNCTION add_audit_package_group_entry_v2(p_origin text, p_channel text, p_package_ids bigint[], p_operation origin_package_operation, p_trigger package_channel_trigger, p_requester_id bigint, p_requester_name text, p_group_id bigint) RETURNS SETOF audit_package_group
    LANGUAGE sql
    AS $$
INSERT INTO audit_package_group (origin_id, channel_id, package_ids, operation, trigger, requester_id, requester_name, group_id)
VALUES (
    (SELECT id FROM origins where name = p_origin),
    (SELECT id FROM get_origin_channel_v1(p_origin, p_channel)),
    p_package_ids, p_operation, p_trigger, p_requester_id, p_requester_name, p_group_id)
RETURNING *;
$$;
