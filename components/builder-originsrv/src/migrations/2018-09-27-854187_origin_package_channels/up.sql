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
