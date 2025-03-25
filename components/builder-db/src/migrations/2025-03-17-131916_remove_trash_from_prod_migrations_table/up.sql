DELETE FROM __diesel_schema_migrations WHERE VERSION = '21081103111111';

-- This delete stmt removed a rows that doesn't map to an existing
-- migration and whose presence was breaking the redo and revert
-- subcommands of the `diesel migration` command.
