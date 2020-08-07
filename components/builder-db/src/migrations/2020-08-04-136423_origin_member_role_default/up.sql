ALTER TABLE origin_members ALTER COLUMN member_role TYPE VARCHAR(255);
ALTER TABLE origin_members ALTER COLUMN member_role DROP DEFAULT;

DROP TYPE IF EXISTS origin_member_role;

CREATE TYPE origin_member_role AS ENUM (
 'readonly_member', 'member', 'maintainer', 'administrator', 'owner'
);
ALTER TABLE origin_members ALTER COLUMN member_role TYPE origin_member_role USING (member_role::origin_member_role);

ALTER TABLE origin_members ALTER COLUMN member_role SET DEFAULT 'readonly_member';
