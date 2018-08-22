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

use diesel::pg::PgConnection;
use diesel::query_dsl::RunQueryDsl;
use diesel::sql_query;

use error::{Error, Result};
use pool::Pool;

pub fn setup_ids(conn: &PgConnection) -> Result<()> {
    sql_query(
        r#"CREATE OR REPLACE FUNCTION next_id_v1(sequence_id regclass, OUT result bigint) AS $$
                DECLARE
                    our_epoch bigint := 1409266191000;
                    seq_id bigint;
                    now_millis bigint;
                BEGIN
                    SELECT nextval(sequence_id) % 1024 INTO seq_id;
                    SELECT FLOOR(EXTRACT(EPOCH FROM clock_timestamp()) * 1000) INTO now_millis;
                    result := (now_millis - our_epoch) << 23;
                    result := result | (seq_id << 13);
                END;
                $$ LANGUAGE PLPGSQL;"#,
    ).execute(conn)
        .unwrap();
    Ok(())
}

pub fn validate_shard_migration(pool: &Pool) -> Result<()> {
    let conn = pool.get()?;

    match conn.query("SELECT shard_migration_complete FROM flags;", &[]) {
        Ok(flag_rows) => {
            if flag_rows.is_empty() {
                match conn.query("SELECT n.nspname FROM pg_catalog.pg_namespace n WHERE n.nspname !~ '^pg_' AND n.nspname <> 'information_schema' AND n.nspname LIKE 'shard_%';", &[]) {
                    Ok(rows) => {
                        // No rows here means there are no shards, so it must be a brand new database
                        if rows.is_empty() {
                            conn.execute("INSERT INTO flags (shard_migration_complete) VALUES('t');", &[]).unwrap();
                            return Ok(());
                        } else {
                            return Err(Error::ShardMigrationIncomplete);
                        }
                    }
                    Err(e) => {
                        error!(
                            "Error checking if shards exist. e = {:?}",
                            e
                        );
                        return Err(Error::ShardMigrationIncomplete);
                    }
                }
            }

            let row = flag_rows.get(0);
            let complete: bool = row.get("shard_migration_complete");

            if complete {
                Ok(())
            } else {
                Err(Error::ShardMigrationIncomplete)
            }
        }
        Err(e) => {
            error!(
                "Error checking if the shard migration is complete. e = {:?}",
                e
            );
            Err(Error::ShardMigrationIncomplete)
        }
    }
}
