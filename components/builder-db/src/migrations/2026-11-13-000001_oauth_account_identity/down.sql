DROP INDEX accounts_oauth_identity_unique;
ALTER TABLE accounts DROP CONSTRAINT accounts_oauth_identity_pair;
ALTER TABLE accounts DROP COLUMN extern_id;
ALTER TABLE accounts DROP COLUMN provider;
