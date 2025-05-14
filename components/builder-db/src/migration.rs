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

use std::io;

use diesel::{pg::PgConnection,
             query_dsl::RunQueryDsl,
             result::{Error as Dre,
                    QueryResult},
             sql_query,
             Connection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

use crate::error::{Result,Error};

/// Embed all migrations from src/migrations into a single constant
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/migrations");

/// Run setup and then all pending migrations
pub fn setup(conn: &mut PgConnection) -> QueryResult<()> {
    conn.transaction::<(), Dre, _>(|conn| {
        setup_ids(conn)?;
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(Dre::QueryBuilderError)?;
        Ok(())
    })?;
    Ok(())
}

pub fn setup_ids(conn: &mut PgConnection) -> QueryResult<()> {
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
    )
    .execute(conn)?;
    Ok(())
}
