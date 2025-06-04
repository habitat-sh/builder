CREATE TYPE package_channel_operation AS ENUM (
  'promote',
  'demote'
);

ALTER TABLE audit_package
  ALTER COLUMN operation SET DATA TYPE package_channel_operation
    USING CASE operation
      WHEN 'promote' THEN 'promote'::package_channel_operation
      WHEN 'demote'  THEN 'demote'::package_channel_operation
      ELSE NULL
    END;