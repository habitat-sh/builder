--- ensure any new owner is set
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

--- ensure that the old owners are now maintainers
UPDATE
  origin_members om
SET
  member_role = 'maintainer'
WHERE
  account_id IN (
    SELECT
      origin_members.account_id
    FROM
      origin_members
      INNER JOIN origins ON (
        origins.name = origin_members.origin
        AND origin_members.member_role = 'owner'
        AND origins.owner_id != origin_members.account_id
      )
);
