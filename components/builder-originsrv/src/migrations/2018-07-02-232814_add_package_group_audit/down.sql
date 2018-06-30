DROP TABLE IF EXISTS audit_package;
DROP TABLE IF EXISTS audit_package_group;
DROP FUNCTION IF EXISTS add_audit_package_entry_v1(bigint, bigint, bigint, smallint, smallint, bigint, text);
DROP FUNCTION IF EXISTS add_audit_package_group_entry_v1(bigint, bigint, bigint[], smallint, smallint, bigint, text, bigint);
