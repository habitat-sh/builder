CREATE OR REPLACE FUNCTION delete_origin_channel_v2(channel_name text, channel_origin text) RETURNS void
    LANGUAGE sql
    AS $$
      DELETE FROM origin_channels
      USING origins
      WHERE origin_channels.name = channel_name
      AND origins.name = channel_origin
$$;
