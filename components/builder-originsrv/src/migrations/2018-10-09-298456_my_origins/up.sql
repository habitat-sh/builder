CREATE OR REPLACE FUNCTION my_origins_with_stats_v1(om_account_id bigint) RETURNS TABLE(id bigint, name text, owner_id bigint, created_at timestamp with time zone, updated_at timestamp with time zone, default_package_visibility text, package_count bigint)
    LANGUAGE sql STABLE
    AS $$
  SELECT o.*, count(op.id) as package_count
  FROM origins o
  INNER JOIN origin_members om ON o.id = om.origin_id
  LEFT OUTER JOIN origin_packages op on o.id = op.origin_id
  WHERE om.account_id = om_account_id
  GROUP BY o.id
  ORDER BY o.name;
$$;
