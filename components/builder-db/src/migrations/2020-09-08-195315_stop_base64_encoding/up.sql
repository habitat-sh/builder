-- Stop base64 encoding our encrypted content; it's already a string
-- anyway.

UPDATE origin_secret_keys
SET body = convert_from(decode(body, 'base64'), 'UTF-8');

UPDATE origin_integrations
SET body = convert_from(decode(body, 'base64'), 'UTF-8');
