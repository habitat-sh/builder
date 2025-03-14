CREATE INDEX IF NOT EXISTS idx_tdeps ON origin_packages USING GIN(tdeps);
CREATE INDEX IF NOT EXISTS idx_build_tdeps ON origin_packages USING GIN(tdeps);

-- These indexes are support the direct sql_query rdep implementation.
-- See 'async fn get_rdeps' components/builder-api/src/server/resources/jobs.rs
