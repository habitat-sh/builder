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

use std::{fmt,
          ops::{Deref,
                DerefMut},
          thread,
          time::Duration};

use postgres::tls;
use r2d2::Pool as R2d2Pool;
use r2d2_postgres::{self,
                    PostgresConnectionManager};

use crate::{config::DataStoreCfg,
            error::{Error,
                    Result}};

#[derive(Clone)]
pub struct Pool {
    inner: R2d2Pool<PostgresConnectionManager<tls::NoTls>>,
}

impl fmt::Debug for Pool {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Pool {{ inner: {:?} }}", self.inner)
    }
}

impl Pool {
    pub fn new(config: &DataStoreCfg) -> Self {
        debug!("Creating new Pool, config: {:?}", config);
        let mut upconfig = postgres::config::Config::new();
        upconfig.user(&config.user);

        loop {
            let conf = format!("{}", config).parse()
                                            .expect("Couldn't parse postgres config.");
            let manager = PostgresConnectionManager::new(conf, tls::NoTls);
            match R2d2Pool::builder()
                .max_size(config.pool_size)
                .connection_timeout(Duration::from_secs(config.connection_timeout_sec))
                .build(manager)
            {
                Ok(pool) => return Pool { inner: pool },
                Err(e) => error!(
                    "Error initializing connection pool to Postgres, will retry: {}",
                    e
                ),
            }
            thread::sleep(Duration::from_millis(config.connection_retry_ms));
        }
    }

    pub fn get(
        &self)
        -> Result<r2d2::PooledConnection<r2d2_postgres::PostgresConnectionManager<tls::NoTls>>>
    {
        let conn = self.inner.get().map_err(Error::ConnectionTimeout)?;
        Ok(conn)
    }
}

impl Deref for Pool {
    type Target = r2d2::Pool<PostgresConnectionManager<tls::NoTls>>;

    fn deref(&self) -> &r2d2::Pool<PostgresConnectionManager<tls::NoTls>> { &self.inner }
}

impl DerefMut for Pool {
    fn deref_mut(&mut self) -> &mut r2d2::Pool<PostgresConnectionManager<tls::NoTls>> {
        &mut self.inner
    }
}
