ALTER TABLE audit_package_group ALTER COLUMN operation SET DATA TYPE origin_package_operation
    USING CASE operation
        WHEN 0 THEN 'promote'
        WHEN 1 THEN 'demote'
        ELSE NULL
    END :: origin_package_operation;

ALTER TABLE audit_package_group ALTER COLUMN trigger SET DATA TYPE package_channel_trigger
    USING CASE trigger
        WHEN 0 THEN 'unknown'
        WHEN 1 THEN 'builder_ui'
        WHEN 2 THEN 'hab_client'
        ELSE NULL
    END :: package_channel_trigger;

ALTER TABLE origin_projects ALTER COLUMN visibility DROP DEFAULT;

ALTER TABLE origin_projects ALTER COLUMN visibility SET DATA TYPE origin_package_visibility
    USING visibility :: origin_package_visibility;

ALTER TABLE origin_projects ALTER COLUMN visibility SET DEFAULT 'public'::origin_package_visibility;
