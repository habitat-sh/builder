DELETE FROM
  origin_members T1
WHERE
  EXISTS (
    SELECT
      1
    FROM
      origin_members T2
    WHERE
      T1.account_id = T2.account_id
      AND T1.origin = T2.origin
      AND T1.ctid > T2.ctid
  );

ALTER TABLE
  origin_members
ADD
  CONSTRAINT origin_members_origin_account_id_key UNIQUE(origin, account_id);

ALTER TABLE
  origin_invitations
ADD
  CONSTRAINT origin_invitations_origin_account_id_key UNIQUE(origin, account_id);

