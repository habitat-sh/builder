CREATE TABLE IF NOT EXISTS flags (
    shard_migration_complete bool DEFAULT false,
    created_at timestamptz DEFAULT now(),
    updated_at timestamptz
);
