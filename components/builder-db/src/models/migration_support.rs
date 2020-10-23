//! Accessory code to add to various models to support various
//! non-SQL, active data migrations.
//!
//! The idea is that these are not intended for use in "normal" code,
//! wil be removed at some point in the future, and thus should be
//! sequestered apart as "special" code

use crate::models::keys as db_keys;
use diesel::{self,
             pg::PgConnection,
             result::QueryResult,
             ExpressionMethods,
             QueryDsl,
             RunQueryDsl};
use habitat_core::crypto::keys as core_keys;

impl db_keys::OriginPrivateEncryptionKey {
    /// Encrypts any unencrypted secret encryption keys in the
    /// database with the given Builder secret encryption key.
    ///
    /// Returns the number of updated rows for user feedback purposes.
    ///
    /// Should be run in a transaction!
    pub fn encrypt_unencrypted_keys(conn: &PgConnection,
                                    encryption_key: &core_keys::BuilderSecretEncryptionKey)
                                    -> QueryResult<u32> {
        use crate::schema::key::origin_private_encryption_keys::dsl::*;

        let mut updated_rows = 0;
        for row in origin_private_encryption_keys.for_update()
                                                 .get_results::<Self>(conn)?
        {
            // If contents are not encrypted, then encrypt and update. The
            // key can't be parsed if it's encrypted.
            if row.body
                  .parse::<core_keys::OriginSecretEncryptionKey>()
                  .is_ok()
            {
                let encrypted = encryption_key.encrypt(&row.body);
                diesel::update(origin_private_encryption_keys.filter(id.eq(row.id)))
                    .set((body.eq(&encrypted.to_string()),))
                    .get_result::<Self>(conn)?;
                updated_rows += 1;
            }
        }
        Ok(updated_rows)
    }
}
