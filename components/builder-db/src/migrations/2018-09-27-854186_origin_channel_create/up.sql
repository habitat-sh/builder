CREATE OR REPLACE FUNCTION insert_origin_channel_v2(occ_origin text, occ_owner_id bigint, occ_name text) RETURNS SETOF origin_channels
    LANGUAGE sql
    AS $$
    INSERT INTO origin_channels (origin_id, owner_id, name)
        VALUES ((select id from origins where name = occ_origin), occ_owner_id, occ_name)
        RETURNING *;
$$;
