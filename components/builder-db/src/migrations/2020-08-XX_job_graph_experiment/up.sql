CREATE TYPE job_exec_state AS ENUM (
  'pending',
  'schedulable',
  'eligible',
  'dispatched',
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
CREATE INDEX dependencies ON job_graph USING GIN(dependencies);

-- This index might be combined with another field (maybe group_id?)
CREATE INDEX state ON job_graph (job_state);

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
               AND d.id = ANY (j.dependencies)
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

-- TODO:
-- add foreign key constraint on group_id
-- is id tied to job.id?
-- is dependencies FK constrained to self?
--
-- Index on group_id probably
-- Index on group_id, job_state, dependencies probably
-- Index on job_state, target_platform (or reverse?)
