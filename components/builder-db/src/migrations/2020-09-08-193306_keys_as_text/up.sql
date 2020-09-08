-- Previously, keys were stored as hex-escaped BYTEA fields. We want
-- that to be TEXT, though.
--
-- If we simply turn this into TEXT, however, the escape character and
-- hexadecimal encoding comes along for the ride. To fix this, we have
-- chop off the leading '\x', decode the hexadecimal byte string, and
-- then convert it all into a UTF-8 string. Depending on the table,
-- that's going to either be a base64-encoded string, or a key.

ALTER TABLE origin_public_keys ALTER COLUMN body TYPE TEXT;
UPDATE origin_public_keys
SET body = convert_from(decode(ltrim(body, '\x'), 'hex'), 'UTF-8');

ALTER TABLE origin_secret_keys ALTER COLUMN body TYPE TEXT;
UPDATE origin_secret_keys
SET body = convert_from(decode(ltrim(body, '\x'), 'hex'), 'UTF-8');

ALTER TABLE origin_public_encryption_keys ALTER COLUMN body TYPE TEXT;
UPDATE origin_public_encryption_keys
SET body = convert_from(decode(ltrim(body, '\x'), 'hex'), 'UTF-8');

ALTER TABLE origin_private_encryption_keys ALTER COLUMN body TYPE TEXT;
UPDATE origin_private_encryption_keys
SET body = convert_from(decode(ltrim(body, '\x'), 'hex'), 'UTF-8');
