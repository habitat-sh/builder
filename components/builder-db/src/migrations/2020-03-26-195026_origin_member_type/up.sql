CREATE TYPE origin_member_role AS ENUM (
  'member', 'maintainer', 'administrator', 'owner'
);

ALTER TABLE
  origin_members
ADD
  COLUMN member_role origin_member_role NOT NULL DEFAULT 'maintainer';

UPDATE
  origin_members om
SET
  member_role = 'owner'
WHERE
  account_id IN (
    SELECT
      owner_id
    FROM
      origins o
    WHERE
      om.account_id = o.owner_id
      AND om.origin = o.name
  );
