DROP SEQUENCE IF EXISTS account_tokens_id_seq CASCADE;
DROP TABLE IF EXISTS account_tokens CASCADE;
DROP FUNCTION IF EXISTS insert_account_token_v1(bigint, text);
DROP FUNCTION IF EXISTS get_account_tokens_v1(bigint, boolean);
DROP FUNCTION IF EXISTS revoke_account_token_v1(bigint, bigint);
