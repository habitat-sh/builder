CREATE OR REPLACE FUNCTION delete_origin_channel_v2(channel_name text, channel_origin text) RETURNS void
    LANGUAGE sql
    AS $$
      DELETE origin_channels FROM origin_channels
      JOIN origins
      WHERE origin_channel.name = channel_name
      AND origins.name = channel_origin;
    RETURNING *
$$;
