use crate::{db::models::keys::OriginPrivateSigningKey,
            server::error::{Error,
                            Result}};
use builder_core::crypto;
use diesel::pg::PgConnection;
use habitat_core::crypto::keys::{Key,
                                 KeyCache};
use std::time::Instant;

// This value was arbitrarily chosen and might need some tuning
const KEY_MIGRATION_CHUNK_SIZE: i64 = 100;

pub fn migrate_to_encrypted(conn: &PgConnection, key_cache: &KeyCache) -> Result<()> {
    let start_time = Instant::now();
    let mut updated_keys = 0;
    let mut skipped_keys = 0;
    let builder_secret_key = key_cache.latest_builder_key()?;
    let mut next_id: i64 = 0;

    loop {
        let skeys = OriginPrivateSigningKey::list_unencrypted(next_id,
                                                              KEY_MIGRATION_CHUNK_SIZE,
                                                              conn).map_err(Error::DieselError)?;
        warn!("migrate_to_encrypted found {}/{} keys requested",
              skeys.len(),
              KEY_MIGRATION_CHUNK_SIZE);
        if skeys.is_empty() {
            break;
        };

        for skey in skeys {
            next_id = skey.id;
            if skey.encryption_key_rev.is_none() {
                let unencrypted_key = skey.body;
                let (encrypted_key, _revision) =
                    crypto::encrypt_with_key(&builder_secret_key, unencrypted_key);

                OriginPrivateSigningKey::update_key(skey.id,
                                                    encrypted_key.as_bytes(),
                                                    &builder_secret_key.named_revision()
                                                                       .revision(),
                                                    conn).map_err(Error::DieselError)?;
                updated_keys += 1;
            } else {
                skipped_keys += 1;
            }
        }
    }

    warn!("migrate_to_encrypted complete in {} sec, updated {}, skipped {} as already updated",
          start_time.elapsed().as_secs_f64(),
          updated_keys,
          skipped_keys);
    Ok(())
}
