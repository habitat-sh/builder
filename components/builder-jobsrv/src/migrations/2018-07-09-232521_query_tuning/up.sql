CREATE OR REPLACE FUNCTION count_group_projects_v2 (origin text) RETURNS bigint AS $$
  BEGIN
    RETURN COUNT(*) FROM group_projects WHERE project_ident LIKE (origin || '/%');
  END
$$ LANGUAGE plpgsql STABLE;

ALTER TABLE IF EXISTS graph_packages ADD ident_array text[];
CREATE INDEX IF NOT EXISTS graph_packages_ident_array ON graph_packages (ident_array);

-- Normally I try to reserve migrations for schema changes only, but I think
-- this is ok, since it's idempotent.
UPDATE graph_packages SET ident_array=regexp_split_to_array(ident, '/');

CREATE OR REPLACE FUNCTION count_graph_packages_v2 (origin text) RETURNS bigint AS $$
BEGIN
  RETURN COUNT(*) FROM graph_packages WHERE ident_array[1]=origin;
END
$$ LANGUAGE plpgsql STABLE;

CREATE OR REPLACE FUNCTION count_unique_graph_packages_v2 (
  op_origin text
) RETURNS bigint
LANGUAGE SQL STABLE AS $$
  SELECT COUNT(DISTINCT ident_array[2]) AS total
  FROM graph_packages
  WHERE ident_array[1] = op_origin
$$;

CREATE OR REPLACE FUNCTION upsert_graph_package_v2 (
  in_ident text,
  in_deps text[],
  in_target text
) RETURNS SETOF graph_packages AS $$
  BEGIN
    RETURN QUERY INSERT INTO graph_packages (ident, deps, target, ident_array)
    VALUES (in_ident, in_deps, in_target, regexp_split_to_array(in_ident, '/'))
    ON CONFLICT(ident)
    DO UPDATE SET deps=in_deps, target=in_target RETURNING *;
    RETURN;
  END
$$ LANGUAGE plpgsql VOLATILE;
