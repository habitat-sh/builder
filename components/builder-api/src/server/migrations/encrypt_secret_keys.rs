//! Ensure that existing origin secret encryption keys are themselves
//! encrypted at rest in the database.

use crate::server::error::{Error,
                           Result};
use diesel::{connection::Connection,
             pg::PgConnection};
use habitat_builder_db::models::keys as db_keys;
use habitat_core::crypto::keys::KeyCache;
use std::time::Instant;

/// Perform the actual migration of data.
pub fn run(conn: &PgConnection, key_cache: &KeyCache) -> Result<()> {
    let start_time = Instant::now();
    let builder_encryption_key = key_cache.latest_builder_key()?;

    let updated_rows = conn.transaction::<_, Error, _>(|| {
                           Ok(db_keys::OriginPrivateEncryptionKey::encrypt_unencrypted_keys(conn,
                                                                      &builder_encryption_key)?)
                       })?;

    warn!("secret key encryption completed in {} sec; updated {} rows",
          start_time.elapsed().as_secs_f64(),
          updated_rows);

    Ok(())
}
