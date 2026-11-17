ALTER TYPE origin_operation ADD VALUE IF NOT EXISTS 'key_create';
ALTER TYPE origin_operation ADD VALUE IF NOT EXISTS 'key_public_upload';
ALTER TYPE origin_operation ADD VALUE IF NOT EXISTS 'key_secret_upload';
ALTER TYPE origin_operation ADD VALUE IF NOT EXISTS 'key_secret_delete';
