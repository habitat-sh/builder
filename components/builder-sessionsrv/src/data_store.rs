// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
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

//! The PostgreSQL backend for the Account Server.

embed_migrations!("src/migrations");

use std::io;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use db::config::DataStoreCfg;
use db::diesel_pool::DieselPool;
use db::migration::setup_ids;
use db::pool::Pool;
use diesel::result::Error as Dre;
use diesel::Connection;
use postgres;
use protobuf;
use protocol::sessionsrv;

use error::{SrvError, SrvResult};

#[derive(Clone)]
pub struct DataStore {
    pub pool: Pool,
    pub diesel_pool: DieselPool,
}

impl DataStore {
    pub fn new(cfg: &DataStoreCfg) -> SrvResult<DataStore> {
        let pool = Pool::new(&cfg)?;
        let diesel_pool = DieselPool::new(&cfg)?;
        Ok(DataStore { pool, diesel_pool })
    }

    // For testing only
    pub fn from_pool(pool: Pool, diesel_pool: DieselPool, _: Arc<String>) -> SrvResult<DataStore> {
        Ok(DataStore { pool, diesel_pool })
    }

    pub fn setup(&self) -> SrvResult<()> {
        let conn = self.diesel_pool.get_raw()?;
        let _ = conn.transaction::<_, Dre, _>(|| {
            setup_ids(&*conn).unwrap();
            embedded_migrations::run_with_output(&*conn, &mut io::stdout()).unwrap();
            Ok(())
        });
        Ok(())
    }

    pub fn account_find_or_create(
        &self,
        msg: &sessionsrv::AccountFindOrCreate,
    ) -> SrvResult<sessionsrv::Account> {
        let conn = self.pool.get()?;
        let rows = conn.query(
            "SELECT * FROM select_or_insert_account_v1($1, $2)",
            &[&msg.get_name(), &msg.get_email()],
        )?;
        let row = rows.get(0);
        Ok(self.row_to_account(row))
    }

    pub fn update_account(&self, account_update: &sessionsrv::AccountUpdate) -> SrvResult<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "SELECT update_account_v1($1, $2)",
            &[
                &(account_update.get_id() as i64),
                &account_update.get_email(),
            ],
        ).map_err(SrvError::AccountUpdate)?;
        Ok(())
    }

    pub fn create_account(
        &self,
        account_create: &sessionsrv::AccountCreate,
    ) -> SrvResult<sessionsrv::Account> {
        let conn = self.pool.get()?;
        let rows =
            conn.query(
                "SELECT * FROM select_or_insert_account_v1($1, $2)",
                &[&account_create.get_name(), &account_create.get_email()],
            ).map_err(SrvError::AccountCreate)?;
        let row = rows.get(0);
        let account = self.row_to_account(row);
        Ok(account)
    }

    pub fn get_account(
        &self,
        account_get: &sessionsrv::AccountGet,
    ) -> SrvResult<Option<sessionsrv::Account>> {
        let conn = self.pool.get()?;
        let rows =
            conn.query(
                "SELECT * FROM get_account_by_name_v1($1)",
                &[&account_get.get_name()],
            ).map_err(SrvError::AccountGet)?;
        if rows.len() != 0 {
            let row = rows.get(0);
            Ok(Some(self.row_to_account(row)))
        } else {
            Ok(None)
        }
    }

    pub fn get_account_by_id(
        &self,
        account_get_id: &sessionsrv::AccountGetId,
    ) -> SrvResult<Option<sessionsrv::Account>> {
        let conn = self.pool.get()?;
        let rows =
            conn.query(
                "SELECT * FROM get_account_by_id_v1($1)",
                &[&(account_get_id.get_id() as i64)],
            ).map_err(SrvError::AccountGetById)?;
        if rows.len() != 0 {
            let row = rows.get(0);
            Ok(Some(self.row_to_account(row)))
        } else {
            Ok(None)
        }
    }

    pub fn create_account_token(
        &self,
        account_token_create: &sessionsrv::AccountTokenCreate,
    ) -> SrvResult<sessionsrv::AccountToken> {
        let conn = self.pool.get()?;
        let rows =
            conn.query(
                "SELECT * FROM insert_account_token_v1($1, $2)",
                &[
                    &(account_token_create.get_account_id() as i64),
                    &account_token_create.get_token(),
                ],
            ).map_err(SrvError::AccountTokenCreate)?;
        let row = rows.get(0);
        let account = self.row_to_account_token(row);
        Ok(account)
    }

    pub fn get_account_tokens(
        &self,
        account_tokens_get: &sessionsrv::AccountTokensGet,
    ) -> SrvResult<sessionsrv::AccountTokens> {
        let conn = self.pool.get()?;
        let rows = &conn
            .query(
                "SELECT * FROM get_account_tokens_v1($1)",
                &[&(account_tokens_get.get_account_id() as i64)],
            )
            .map_err(SrvError::AccountTokensGet)?;

        let mut account_tokens = sessionsrv::AccountTokens::new();
        let mut tokens = protobuf::RepeatedField::new();
        for row in rows {
            let account_token = self.row_to_account_token(row);
            tokens.push(account_token);
        }
        account_tokens.set_tokens(tokens);
        Ok(account_tokens)
    }

    pub fn get_account_token(
        &self,
        account_token_get: &sessionsrv::AccountTokenGet,
    ) -> SrvResult<sessionsrv::AccountToken> {
        let conn = self.pool.get()?;
        let rows = &conn
            .query(
                "SELECT * FROM get_account_token_with_id_v1($1)",
                &[&(account_token_get.get_id() as i64)],
            )
            .map_err(SrvError::AccountTokensGet)?;

        assert!(rows.len() == 1);
        let row = rows.get(0);
        let account = self.row_to_account_token(row);
        Ok(account)
    }

    pub fn revoke_account_token(
        &self,
        account_token_revoke: &sessionsrv::AccountTokenRevoke,
    ) -> SrvResult<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "SELECT * FROM revoke_account_token_v1($1)",
            &[&(account_token_revoke.get_id() as i64)],
        ).map_err(SrvError::AccountTokenRevoke)?;

        Ok(())
    }

    fn row_to_account(&self, row: postgres::rows::Row) -> sessionsrv::Account {
        let mut account = sessionsrv::Account::new();
        let id: i64 = row.get("id");
        account.set_id(id as u64);
        account.set_email(row.get("email"));
        account.set_name(row.get("name"));
        account
    }

    fn row_to_account_token(&self, row: postgres::rows::Row) -> sessionsrv::AccountToken {
        let mut account_token = sessionsrv::AccountToken::new();
        let id: i64 = row.get("id");
        account_token.set_id(id as u64);
        let account_id: i64 = row.get("account_id");
        account_token.set_account_id(account_id as u64);
        account_token.set_token(row.get("token"));

        let created_at = row.get::<&str, DateTime<Utc>>("created_at");
        account_token.set_created_at(created_at.to_rfc3339());

        account_token
    }
}
