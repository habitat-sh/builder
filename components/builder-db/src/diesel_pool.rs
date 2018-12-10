// Copyright (c) 2018 Chef Software Inc. and/or applicable contributors
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

use std::thread;
use std::time::Duration;

use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

use crate::config::DataStoreCfg;
use crate::error::Result;

type PgPool = Pool<ConnectionManager<PgConnection>>;

type PgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct DbPool(pub PgPool);

impl DbPool {
    pub fn new(config: &DataStoreCfg) -> Self {
        debug!("Creating new DbPool, config: {:?}", config);
        loop {
            let manager = ConnectionManager::<PgConnection>::new(config.to_string());
            match Pool::builder()
                .max_size(config.pool_size)
                .connection_timeout(Duration::from_secs(config.connection_timeout_sec))
                .build(manager)
            {
                Ok(pool) => return DbPool(pool),
                Err(e) => error!(
                    "Error initializing connection pool to Postgres, will retry: {}",
                    e
                ),
            }
            thread::sleep(Duration::from_millis(config.connection_retry_ms));
        }
    }

    pub fn get_conn(&self) -> Result<PgPooledConnection> {
        match self.0.get() {
            Ok(conn) => Ok(conn),
            Err(e) => Err(e.into()),
        }
    }
}
