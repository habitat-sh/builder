CREATE TYPE origin_package_operation AS ENUM ('promote', 'demote');
CREATE TYPE package_channel_trigger AS ENUM ('unknown', 'builder_ui', 'hab_client');

ALTER TABLE audit_package ALTER COLUMN operation SET DATA TYPE origin_package_operation
    USING CASE operation
        WHEN 0 THEN 'promote'
        WHEN 1 THEN 'demote'
        ELSE NULL
    END :: origin_package_operation;

ALTER TABLE audit_package ALTER COLUMN trigger SET DATA TYPE package_channel_trigger
    USING CASE trigger
        WHEN 0 THEN 'unknown'
        WHEN 1 THEN 'builder_ui'
        WHEN 2 THEN 'hab_client'
        ELSE NULL
    END :: package_channel_trigger;
