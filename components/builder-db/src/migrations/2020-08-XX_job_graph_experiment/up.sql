CREATE TYPE job_exec_state AS ENUM (
  'pending',
  'waiting_on_dependency',
  'ready',
  'running',
  'complete',
  'job_failed',
  'dependency_failed',
  'cancel_pending',
  'cancel_complete'
);

CREATE SEQUENCE IF NOT EXISTS job_graph_id_seq;

CREATE TABLE IF NOT EXISTS job_graph (
    id BIGINT DEFAULT nextval('job_graph_id_seq') PRIMARY KEY NOT NULL,
    group_id BIGINT NOT NULL,
    job_state job_exec_state,
    plan_ident TEXT,
    manifest_ident TEXT,
    as_built_ident TEXT,
    dependencies BIGINT[] NOT NULL,
    waiting_on_count INTEGER NOT NULL,
    target_platform TEXT NOT NULL,
    -- may insert some more prioritzation stuff, around groups, etc.
    created_at timestamp WITH time zone DEFAULT now() NOT NULL,
    updated_at timestamp WITH time zone DEFAULT now() NOT NULL
);

-- diesel trigger to manage update
SELECT diesel_manage_updated_at('job_graph');

-- This is required for fast search inside the array
-- It might get large, maybe we should create partial index
-- either filtered on job state or a separate active/archived flag
CREATE EXTENSION intarray;
CREATE INDEX ON job_graph USING GIN(dependencies);

-- This index might be combined with another field (maybe group_id?)
CREATE INDEX state ON job_graph (job_state);

-- TODO Possible index
-- target, job_exec_state for count_ready_by_target
-- group_id, job_exec_state?

----------------------------------------------
--
-- Compute transitively expanded rdeps for id
--
-- Performance note:
-- While it might seem easier to write "re.id = ANY(g.dependencies))" below
-- the gin index doesn't supprt that function, and is hella slow.
-- Using @> is vastly faster. (e.g. 30k entries, 7000 ms becomes 15ms)
--
-- needs gin index on dependencies 
CREATE OR REPLACE FUNCTION t_rdeps_for_id(job_graph_id BIGINT)
RETURNS SETOF BIGINT
AS $$
DECLARE
  failed_count integer;
BEGIN
  -- Recursively expand all things that depend on me
  RETURN QUERY (
    WITH RECURSIVE re(id) AS (
      SELECT g.id FROM job_graph g WHERE g.dependencies @> array[job_graph_id]::bigint[]
    UNION
      SELECT g.id
      FROM job_graph g, re
      WHERE g.dependencies @> array[re.id]::bigint[] )
    SELECT * FROM re);
END
$$ LANGUAGE PLPGSQL;

----------------------------------------------
--
-- Compute transitively expanded deps for id
--
-- Not super performant (28ms with 30k entries), but
-- not on critical path either.
-- needs gin index on dependencies
CREATE OR REPLACE FUNCTION t_deps_for_id(job_graph_id BIGINT)
RETURNS SETOF BIGINT
AS $$
DECLARE
  failed_count integer;
BEGIN
  -- Recursively expand all things that depend on me
  RETURN QUERY (
    WITH RECURSIVE re(id) AS (
      SELECT UNNEST(g.dependencies) FROM job_graph g WHERE g.id = job_graph_id
    UNION
      SELECT UNNEST(g.dependencies)
      FROM job_graph g, re
      WHERE g.id = re.id)
    SELECT * FROM re);
END
$$ LANGUAGE PLPGSQL;

-- as above, but faster (3ms) because group_id filter
-- needs index on group_id
CREATE OR REPLACE FUNCTION t_deps_for_id_group(in_id BIGINT, in_group_id BIGINT)
RETURNS SETOF BIGINT
AS $$
BEGIN
  -- Recursively expand all things that depend on me
  RETURN QUERY (
    WITH RECURSIVE re(id) AS (
      SELECT UNNEST(g.dependencies) FROM job_graph g where g.id = in_id
    AND g.group_id = in_group_id
    UNION
      SELECT UNNEST(g2.dependencies)
      FROM job_graph g2, re
      WHERE g2.id = re.id  AND g2.group_id = in_group_id )
    SELECT * FROM re);
END
$$ LANGUAGE PLPGSQL;

--
--
-- This is too slow for production use, but is intended as a debugging aid
CREATE OR REPLACE VIEW job_graph_completed AS
SELECT *,
  (SELECT array_cat(array[]::BIGINT[], array_agg(d.id)) -- array_agg alone fills things with nulls when no deps
   FROM job_graph AS d
   WHERE d.id = ANY (j.dependencies)
     AND d.job_state = 'complete') AS complete
FROM job_graph AS j;

-- Also very slow, but useful for recovery
-- note doesn't reset state if dependencies aren't complete
CREATE OR REPLACE FUNCTION job_graph_fixup_waiting_on_count() RETURNS integer AS $$
DECLARE
i_count integer;
BEGIN
UPDATE job_graph
SET waiting_on_count = subquery.remaining
FROM (SELECT id, (cardinality(k.dependencies) - complete_count) AS remaining
      FROM (
            SELECT *,
               (
               SELECT
               count(*)
               FROM
               job_graph AS d
               WHERE
               d.group_id = d.group_id
               AND j.dependencies @> array[d.id]::bigint[]
               AND d.job_state = 'complete')
             AS complete_count
             FROM job_graph AS j)
       AS k) AS subquery
WHERE
 job_graph.id = subquery.id
AND  waiting_on_count != subquery.remaining;
GET DIAGNOSTICS i_count = ROW_COUNT;
RETURN i_count;
END
$$ LANGUAGE PLPGSQL;


-- Mark a job complete and update the jobs that depend on it
-- If a job has zero dependencies, mark it ready to be run
--
-- We rely on this being atomic (like all functions in postgres)
-- It might be better to write this as a diesel transaction, but it's kinda complex
--
CREATE OR REPLACE FUNCTION job_graph_mark_complete(job_graph_id BIGINT) RETURNS integer AS $$
DECLARE
  i_count integer;
BEGIN
  -- Decrement count of the things that depend on us
  -- TODO: Consider limiting this update to jobs 'ready'
  UPDATE job_graph
    SET waiting_on_count = waiting_on_count - 1
    FROM (SELECT id
          FROM job_graph AS d
          WHERE d.dependencies @> array[job_graph_id]::bigint[]
         ) as deps
    WHERE job_graph.id = deps.id;

  UPDATE job_graph
    SET job_state = 'ready'
    WHERE waiting_on_count = 0
    AND job_state = 'waiting_on_dependency';

  -- postgres magic to get number of altered rows in prior query
  GET DIAGNOSTICS i_count = ROW_COUNT;

  -- Mark this job complete
  -- TODO: Consider limiting this update to jobs 'Pending'
  UPDATE job_graph SET job_state = 'complete'
  WHERE id = job_graph_id;

  RETURN i_count;
END
$$ LANGUAGE PLPGSQL;


-- Mark a job complete and recursively update the jobs that depend on it
--
-- We rely on this being atomic (like all functions in postgres)
-- It might be better to write this as a diesel transaction, but it's kinda complex,
-- we'd probably have to write the recursion as multiple calls, which gets messy
--
CREATE OR REPLACE FUNCTION job_graph_mark_failed(job_graph_id BIGINT) RETURNS integer AS $$
DECLARE
  failed_count integer;
BEGIN
  -- Recursively expand all things that depend on me
  -- this maybe could be DRY with the t_rdeps_for_id call above
  WITH RECURSIVE re(id) AS (
    SELECT g.id FROM job_graph g where g.dependencies @> array[job_graph_id]::bigint[]
  UNION
    SELECT g.id
    FROM job_graph g, re
    WHERE g.dependencies @> array[re.id]::bigint[])
  UPDATE job_graph SET job_state = 'dependency_failed'
    WHERE id IN (SELECT id from re)
    AND (job_state = 'waiting_on_dependency' OR job_state = 'ready');

  GET DIAGNOSTICS failed_count = ROW_COUNT;

  -- Mark this job complete
  -- TODO: Consider limiting this update to jobs 'Pending'
  UPDATE job_graph SET job_state = 'job_failed'
  WHERE id = job_graph_id;

  RETURN failed_count;
END
$$ LANGUAGE PLPGSQL;



-- TODO:
-- add foreign key constraint on group_id
-- is id tied to job.id?
-- is dependencies FK constrained to self?
--
-- Index on group_id probably
-- Index on group_id, job_state, dependencies probably
-- Index on job_state, target_platform (or reverse?)
