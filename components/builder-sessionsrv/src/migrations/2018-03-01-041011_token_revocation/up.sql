CREATE OR REPLACE FUNCTION get_account_token_with_id_v1 (
  p_id bigint
) RETURNS SETOF account_tokens AS $$
    SELECT * FROM account_tokens WHERE id = p_id;
$$ LANGUAGE SQL STABLE;
