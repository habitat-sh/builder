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
             result::Error as Dre,
             sql_query,
             Connection};

use crate::error::Result;

embed_migrations!("src/migrations");

pub fn setup(conn: &PgConnection) -> Result<()> {
    let _ = conn.transaction::<_, Dre, _>(|| {
                    setup_ids(conn).unwrap();
                    embedded_migrations::run_with_output(conn, &mut io::stdout()).unwrap();
                    Ok(())
                });
    Ok(())
}

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
    )
    .execute(conn)
    .unwrap();
    Ok(())
}
