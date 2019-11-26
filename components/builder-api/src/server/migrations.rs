// Copyright (c) 2016 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::PathBuf;

use crate::{db::models::keys::OriginPrivateSigningKey,
            server::error::{Error,
                            Result}};

use diesel::pg::PgConnection;
use time::PreciseTime;

// This value was arbitrarily chosen and might need some tuning
const KEY_MIGRATION_CHUNK_SIZE: i64 = 100;

pub fn migrate_to_encrypted(conn: &PgConnection, key_path: &PathBuf) -> Result<()> {
    let start_time = PreciseTime::now();
    let mut updated_keys = 0;
    let mut skipped_keys = 0;
    let key_pair = builder_core::integrations::get_keypair_helper(key_path)?;
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
                let encrypted_key =
                    builder_core::integrations::encrypt_with_keypair(&key_pair, &unencrypted_key)?;

                OriginPrivateSigningKey::update_key(skey.id,
                                                    encrypted_key.as_bytes(),
                                                    &key_pair.rev,
                                                    conn).map_err(Error::DieselError)?;
                updated_keys += 1;
            } else {
                skipped_keys += 1;
            }
        }
    }

    let end_time = PreciseTime::now();
    warn!("migrate_to_encrypted complete in {} sec, updated {}, skipped {} as already updated",
          start_time.to(end_time),
          updated_keys,
          skipped_keys);
    Ok(())
}
