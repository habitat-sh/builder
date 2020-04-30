--
-- We modify the PARTITION BY include op.origin and op.name. The common uses of this view
-- are SELECTs with WHERE clause constraints on origin and name, and without those the
-- query planner is unable to see that it's safe to filter on those *before* joining. That
-- in turn prevents the use of indicies early in the construction of the join, and we end
-- up moving huge amounts of data (450MB) around because we didn't reduce our set early
-- enough.
--
CREATE OR REPLACE VIEW packages_with_channel_platform AS
 SELECT op.id,
    op.owner_id,
    op.name,
    op.ident,
    op.ident_array,
    op.checksum,
    op.manifest,
    op.config,
    op.target,
    op.deps,
    op.tdeps,
    op.build_deps,
    op.build_tdeps,
    op.exposes,
    op.visibility,
    op.created_at,
    op.updated_at,
    op.origin,
    array_agg(oc.name) OVER w AS channels,
    array_agg(op.target) OVER w AS platforms
   FROM origin_packages op
     JOIN origin_channel_packages ocp ON op.id = ocp.package_id
     JOIN origin_channels oc ON oc.id = ocp.channel_id
  WINDOW w AS (PARTITION BY op.origin, op.name, op.ident);

--
-- With the above partition, we can actually make use of an index on the origin and name
-- Add the target because that's frequently part of the search process as well, and it belongs
-- conceptually as part of the package identifier anyways.
-- We don't index on version or release id, because array elements like ident_array[3] don't seem to be
-- valid syntax in an index.
--
CREATE INDEX IF NOT EXISTS origin_packages_origin_name_target_index ON origin_packages USING btree (origin, name, target);


--
-- We frequently join on package_id between origin_packages and origin_channel_packages
-- This index prevents a sequential search on that join
--
CREATE INDEX IF NOT EXISTS origin_channel_packages_packages_index ON origin_channel_packages USING btree(package_id);

--
-- This was created as a btree, but most of our queries are array contains, which doesn't get
-- any speedup from this index type. Instead we want a GIN (or maybe a GIST)
--
DROP INDEX IF EXISTS origin_packages_ident_array;

CREATE INDEX IF NOT EXISTS origin_packages_ident_array_index ON origin_packages USING GIN (ident_array);

-- Further notes
-- These three changes serve to optimize the query
--
-- EXPLAIN ANALYZE
-- SELECT *, COUNT(*) OVER () FROM (SELECT
-- "packages_with_channel_platform"."id",
-- "packages_with_channel_platform"."owner_id",
-- "packages_with_channel_platform"."name",
-- "packages_with_channel_platform"."ident",
-- "packages_with_channel_platform"."ident_array",
-- "packages_with_channel_platform"."checksum",
-- "packages_with_channel_platform"."manifest",
-- "packages_with_channel_platform"."config",
-- "packages_with_channel_platform"."target",
-- "packages_with_channel_platform"."deps",
-- "packages_with_channel_platform"."tdeps",
-- "packages_with_channel_platform"."build_deps",
-- "packages_with_channel_platform"."build_tdeps",
-- "packages_with_channel_platform"."exposes",
-- "packages_with_channel_platform"."visibility",
-- "packages_with_channel_platform"."created_at",
-- "packages_with_channel_platform"."updated_at",
-- "packages_with_channel_platform"."origin",
-- "packages_with_channel_platform"."channels",
--  "packages_with_channel_platform"."platforms"
--  FROM "packages_with_channel_platform"
--  WHERE "packages_with_channel_platform".ident_array @> '{core, gcc, 9.1.0}'::text[]
--  AND "packages_with_channel_platform"."visibility" = ANY('{public, private, hidden}'::origin_package_visibility[])
--  AND "packages_with_channel_platform"."origin" = 'core' AND "packages_with_channel_platform"."name" = 'gcc'
--  ORDER BY "packages_with_channel_platform"."ident" DESC) t LIMIT 50 OFFSET 0;
--
--
-- Without the above changes EXPLAIN ANALYZE gives:
-- Limit  (cost=84826.79..84826.81 rows=1 width=1421) (actual time=4456.929..4456.930 rows=3 loops=1)
--   ->  WindowAgg  (cost=84826.79..84826.81 rows=1 width=1421) (actual time=4456.927..4456.929 rows=3 loops=1)
--         ->  Sort  (cost=84826.79..84826.79 rows=1 width=1413) (actual time=4456.919..4456.920 rows=3 loops=1)
--               Sort Key: packages_with_channel_platform.ident DESC
--               Sort Method: quicksort  Memory: 31kB
--               ->  Subquery Scan on packages_with_channel_platform  (cost=71856.91..84826.78 rows=1 width=1413) (actual time=4156.508..4456.908 rows=3 loops=1)
--                     Filter: ((packages_with_channel_platform.ident_array @> '{core,gcc,9.1.0}'::text[]) AND (packages_with_channel_platform.origin = 'core'::text) AND (packages_with_channel_platform.name = 'gcc'::text) AND (packages_with_channel_platform.visibility = ANY ('{public,private,hidden}'::origin_package_visibility[])))
--                     Rows Removed by Filter: 309064
--                     ->  WindowAgg  (cost=71856.91..78145.33 rows=314421 width=1413) (actual time=3955.753..4399.871 rows=309067 loops=1)
--                           ->  Sort  (cost=71856.91..72642.96 rows=314421 width=1369) (actual time=3955.730..3997.906 rows=309067 loops=1)
--                                 Sort Key: op.ident
--                                 Sort Method: quicksort  Memory: 589499kB
--                                 ->  Hash Join  (cost=35562.57..43146.60 rows=314421 width=1369) (actual time=299.227..576.649 rows=309067 loops=1)
--                                       Hash Cond: (ocp.channel_id = oc.id)
--                                       ->  Hash Join  (cost=34136.59..40895.18 rows=314421 width=1357) (actual time=287.607..493.531 rows=309067 loops=1)
--                                             Hash Cond: (ocp.package_id = op.id)
--                                             ->  Seq Scan on origin_channel_packages ocp  (cost=0.00..5933.21 rows=314421 width=16) (actual time=0.005..27.531 rows=309067 loops=1)
--                                             ->  Hash  (cost=32209.04..32209.04 rows=154204 width=1349) (actual time=286.726..286.727 rows=154146 loops=1)
--                                                   Buckets: 262144  Batches: 1  Memory Usage: 211584kB
--                                                   ->  Seq Scan on origin_packages op  (cost=0.00..32209.04 rows=154204 width=1349) (actual time=0.005..87.331 rows=154146 loops=1)
--                                       ->  Hash  (cost=922.10..922.10 rows=40310 width=28) (actual time=11.563..11.563 rows=42833 loops=1)
--                                             Buckets: 65536  Batches: 1  Memory Usage: 3217kB
--                                             ->  Seq Scan on origin_channels oc  (cost=0.00..922.10 rows=40310 width=28) (actual time=0.008..5.503 rows=42833 loops=1)
-- Planning Time: 0.518 ms
-- Execution Time: 4485.130 ms
--
-- With the changes we get:
--
-- Limit  (cost=127.64..127.67 rows=1 width=1421) (actual time=0.270..0.272 rows=3 loops=1)
--   ->  WindowAgg  (cost=127.64..127.67 rows=1 width=1421) (actual time=0.269..0.270 rows=3 loops=1)
--         ->  Sort  (cost=127.64..127.65 rows=1 width=1413) (actual time=0.265..0.265 rows=3 loops=1)
--               Sort Key: packages_with_channel_platform.ident DESC
--               Sort Method: quicksort  Memory: 31kB
--               ->  Subquery Scan on packages_with_channel_platform  (cost=126.89..127.63 rows=1 width=1413) (actual time=0.255..0.259 rows=3 loops=1)
--                     Filter: ((packages_with_channel_platform.ident_array @> '{core,gcc,9.1.0}'::text[]) AND (packages_with_channel_platform.visibility = ANY ('{public,private,hidden}'::origin_package_visibility[])))
--                     Rows Removed by Filter: 21
--                     ->  WindowAgg  (cost=126.89..127.34 rows=18 width=1413) (actual time=0.215..0.249 rows=24 loops=1)
--                           ->  Sort  (cost=126.89..126.94 rows=18 width=1369) (actual time=0.206..0.207 rows=24 loops=1)
--                                 Sort Key: op.ident
--                                 Sort Method: quicksort  Memory: 52kB
--                                 ->  Nested Loop  (cost=1.13..126.52 rows=18 width=1369) (actual time=0.037..0.159 rows=24 loops=1)
--                                       ->  Nested Loop  (cost=0.84..120.83 rows=18 width=1357) (actual time=0.031..0.118 rows=24 loops=1)
--                                             ->  Index Scan using origin_packages_origin_name_target_index on origin_packages op  (cost=0.42..40.27 rows=9 width=1349) (actual time=0.021..0.031 rows=12 loops=1)
--                                                   Index Cond: ((origin = 'core'::text) AND (name = 'gcc'::text))
--                                             ->  Index Scan using origin_channel_packages_packages_index on origin_channel_packages ocp  (cost=0.42..8.93 rows=2 width=16) (actual time=0.005..0.006 rows=2 loops=12)
--                                                   Index Cond: (package_id = op.id)
--                                       ->  Index Scan using origin_channels_pkey on origin_channels oc  (cost=0.29..0.32 rows=1 width=28) (actual time=0.001..0.001 rows=1 loops=24)
--                                             Index Cond: (id = ocp.channel_id)
-- Planning Time: 0.729 ms
-- Execution Time: 0.335 ms
--
-- Note the in-memory sort shrank from 589MB to 52kB. That was a function of getting the filters functions applied before the join bloated things. That is what the change in the PARTITION BY clause enabled.
-- Also, the indices added help eliminate sequential scans on origin_channel_packages over the package key, and let us do a index scan on origin_packages to filter by the origin and name
