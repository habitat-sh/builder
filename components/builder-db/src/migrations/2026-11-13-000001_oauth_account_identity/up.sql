ALTER TABLE accounts ADD COLUMN provider text;
ALTER TABLE accounts ADD COLUMN extern_id text;

ALTER TABLE accounts ADD CONSTRAINT accounts_oauth_identity_pair
    CHECK ((provider IS NULL) = (extern_id IS NULL));
CREATE UNIQUE INDEX accounts_oauth_identity_unique
    ON accounts (provider, extern_id)
    WHERE provider IS NOT NULL;
