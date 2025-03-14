-- This file should undo anything in `up.sql

-- INSERT INTO __diesel_schema_migrations ("version", "run_on") VALUES ('21081103111111', '2018-11-06 17:42:38');

-- The up.sql removes bit of trash that found its way into the migrations table
-- in our production instance.  This value was preventing the revert and redo
-- subcommands of `diesel migration` from running.  This down.sql file is used
-- to capture an INSERT that would restore the value that was deleted from the
-- on seemingly rather odd notion that you would want to restore it.  However,
-- since it feels quite odd to restore something that was breaking proper
-- functioning the actual insert is commented out.
