--- ensure owner is set correctly as the result of any origin transfers
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
      AND om.member_role != 'owner'
  );

--- ensure that any incorrectly set owners are transitioned to maintainers
UPDATE
  origin_members
SET
  member_role = 'maintainer'
FROM origins
WHERE
  origins.name = origin_members.origin
  AND origin_members.member_role = 'owner'
  AND origins.owner_id != origin_members.account_id;
