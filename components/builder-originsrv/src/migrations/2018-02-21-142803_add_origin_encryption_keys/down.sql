DROP FUNCTION insert_origin_public_encryption_key_v1 (
  bigint,bigint,text,text,text,bytea
);

DROP FUNCTION get_origin_public_encryption_key_v1 (
  text,text
);

DROP FUNCTION get_origin_public_encryption_key_latest_v1 (
  text
);

DROP FUNCTION get_origin_public_encryption_keys_for_origin_v1 (bigint);

DROP FUNCTION insert_origin_private_encryption_key_v1 (
  bigint,bigint,text,text,text,bytea
);

DROP FUNCTION get_origin_private_encryption_key_v1 (text);

DROP VIEW origins_with_private_encryption_key_full_name_v1;

DROP TABLE IF EXISTS origin_public_encryption_keys;
DROP TABLE IF EXISTS origin_private_encryption_keys;
DROP SEQUENCE origin_public_encryption_key_id_seq;
DROP SEQUENCE origin_private_encryption_key_id_seq;