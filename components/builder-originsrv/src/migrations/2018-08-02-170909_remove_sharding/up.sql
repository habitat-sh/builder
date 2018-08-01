CREATE OR REPLACE FUNCTION my_origins_v2 (
  om_account_id bigint
) RETURNS SETOF origins AS $$
  SELECT o.*
  FROM origins o
  INNER JOIN origin_members om ON o.id = om.origin_id
  WHERE om.account_id = om_account_id
  ORDER BY o.name;
$$ LANGUAGE SQL STABLE;

CREATE OR REPLACE FUNCTION search_all_origin_packages_dynamic_v7 (
  op_query text,
  op_my_origins text
) RETURNS TABLE(ident text) AS $$
  SELECT p.partial_ident[1] || '/' || p.partial_ident[2] AS ident
  FROM (SELECT regexp_split_to_array(op.ident, '/') as partial_ident
    FROM origin_packages op
    WHERE op.ident LIKE ('%' || op_query || '%')
    AND (op.visibility = 'public'
      OR (op.visibility IN ('hidden', 'private') AND op.origin_id IN (SELECT id FROM origins WHERE name = ANY(STRING_TO_ARRAY(op_my_origins, ',')))))) AS p
  GROUP BY (p.partial_ident[1] || '/' || p.partial_ident[2]);
$$ LANGUAGE SQL STABLE;

CREATE OR REPLACE FUNCTION search_all_origin_packages_v6 (
  op_query text,
  op_my_origins text
) RETURNS TABLE(ident text) AS $$
  SELECT op.ident
  FROM origin_packages op
  WHERE op.ident LIKE ('%' || op_query || '%')
  AND (op.visibility = 'public'
    OR (op.visibility IN ('hidden', 'private') AND op.origin_id IN (SELECT id FROM origins WHERE name = ANY(STRING_TO_ARRAY(op_my_origins, ',')))))
  ORDER BY op.ident ASC;
$$ LANGUAGE SQL STABLE;
