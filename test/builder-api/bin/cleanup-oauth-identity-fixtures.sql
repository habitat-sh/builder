BEGIN;

-- Find fixtures by their fixed OAuth subjects as well as their original names:
-- a displaced account may now have a __renamed_account_<id> name.
-- The email marker tracks the displaced legacy fixture, which has no subject.
CREATE TEMP TABLE oauth_identity_fixture_accounts ON COMMIT DROP AS
SELECT id FROM accounts
WHERE name LIKE 'oauth-identity-%'
   OR email = 'oauth-identity-collision-legacy@example.com'
   OR (provider = 'github' AND extern_id IN ('1000001', '1000002', '1000003', '1000004', '1000005', '1000006'))
   OR (provider IN ('gitlab', 'okta') AND extern_id = '1000001');

CREATE TEMP TABLE oauth_identity_fixture_origins ON COMMIT DROP AS
SELECT name FROM origins WHERE name ~ '^oauthidentity[0-9]+$';

DELETE FROM origin_channel_packages WHERE channel_id IN
    (SELECT id FROM origin_channels WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins));
DELETE FROM origin_integrations WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM origin_project_integrations WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM origin_secrets WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM origin_private_encryption_keys WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM origin_public_encryption_keys WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM origin_members WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM origin_channels WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM origin_invitations WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM origin_packages WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM origin_projects WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM origin_public_keys WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM origin_secret_keys WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM audit_package WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM audit_package_group WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM origin_package_settings WHERE origin IN (SELECT name FROM oauth_identity_fixture_origins);
DELETE FROM origins WHERE name IN (SELECT name FROM oauth_identity_fixture_origins);

DELETE FROM account_tokens WHERE account_id IN (SELECT id FROM oauth_identity_fixture_accounts);
DELETE FROM accounts WHERE id IN (SELECT id FROM oauth_identity_fixture_accounts);

COMMIT;
