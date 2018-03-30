CREATE OR REPLACE FUNCTION get_job_groups_for_origin_v2 (
  op_origin text,
  op_limit integer
) RETURNS SETOF groups AS $$
  SELECT *
  FROM groups
  WHERE project_name LIKE (op_origin || '/%')
  ORDER BY created_at DESC
  LIMIT op_limit
$$ LANGUAGE SQL STABLE;
