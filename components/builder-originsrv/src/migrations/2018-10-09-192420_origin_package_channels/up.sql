CREATE TYPE origin_package_operation AS ENUM ('promote', 'demote');
CREATE TYPE package_channel_trigger AS ENUM ('unknown', 'builder_ui', 'hab_client');

ALTER TABLE audit_package ALTER COLUMN operation SET DATA TYPE origin_package_operation
    USING CASE operation
        WHEN 0 THEN 'promote'
        WHEN 1 THEN 'demote'
        ELSE NULL
    END :: origin_package_operation;

ALTER TABLE audit_package ALTER COLUMN trigger SET DATA TYPE package_channel_trigger
    USING CASE trigger
        WHEN 0 THEN 'unknown'
        WHEN 1 THEN 'builder_ui'
        WHEN 2 THEN 'hab_client'
        ELSE NULL
    END :: package_channel_trigger;

CREATE OR REPLACE FUNCTION add_audit_package_entry_v2(p_origin text, p_package text, p_channel text, p_operation origin_package_operation, p_trigger package_channel_trigger, p_requester_id bigint, p_requester_name text) RETURNS SETOF audit_package
    LANGUAGE sql
    AS $$
INSERT INTO audit_package (origin_id, package_id, channel_id, operation, trigger, requester_id, requester_name)
VALUES (
    (SELECT id FROM origins where name = p_origin),
    (SELECT id FROM get_origin_package_v4(p_package, 'public,private,hidden')),
    (SELECT id FROM get_origin_channel_v1(p_origin, p_channel)),
    p_operation, p_trigger, p_requester_id, p_requester_name)
RETURNING *;
$$;

CREATE OR REPLACE FUNCTION promote_origin_package_v2(in_origin text, in_ident text, to_channel text) RETURNS void
    LANGUAGE sql
    AS $$
    INSERT INTO origin_channel_packages (channel_id, package_id)
    VALUES (
        (SELECT id from get_origin_channel_v1(in_origin, to_channel)),
        (SELECT id from get_origin_package_v4(in_ident, 'public,private,hidden'))
    );
$$;
CREATE OR REPLACE FUNCTION demote_origin_package_v2(in_origin text, in_ident text, out_channel text) RETURNS void
    LANGUAGE sql
    AS $$
      DELETE FROM origin_channel_packages
      WHERE channel_id=(SELECT id from get_origin_channel_v1(in_origin, out_channel))
      AND package_id=(SELECT id from get_origin_package_v4(in_ident, 'public,private,hidden'));
$$;
