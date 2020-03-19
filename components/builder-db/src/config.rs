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

use num_cpus;
use percent_encoding::{utf8_percent_encode,
                       AsciiSet,
                       CONTROLS};
use postgres_shared::params::{ConnectParams,
                              Host,
                              IntoConnectParams};
use std::{error::Error,
          fmt};
use url::Url;

// The characters in this set are copied from
// https://docs.rs/percent-encoding/1.0.1/percent_encoding/struct.PATH_SEGMENT_ENCODE_SET.html
const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS.add(b' ')
                                                    .add(b'"')
                                                    .add(b'#')
                                                    .add(b'<')
                                                    .add(b'>')
                                                    .add(b'`')
                                                    .add(b'?')
                                                    .add(b'{')
                                                    .add(b'}')
                                                    .add(b'%')
                                                    .add(b'/');

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct DataStoreCfg {
    pub host:                   String,
    pub port:                   u16,
    pub user:                   String,
    pub password:               Option<String>,
    pub database:               String,
    /// Timing to retry the connection to the data store if it cannot be established
    pub connection_retry_ms:    u64,
    /// How often to cycle a connection from the pool
    pub connection_timeout_sec: u64,
    /// If the datastore connection is under test
    pub connection_test:        bool,
    /// Number of database connections to start in pool.
    pub pool_size:              u32,
    pub ssl_mode:               Option<String>,
    pub ssl_cert:               Option<String>,
    pub ssl_key:                Option<String>,
    pub ssl_root_cert:          Option<String>,
}

impl Default for DataStoreCfg {
    fn default() -> Self {
        DataStoreCfg { host:                   String::from("localhost"),
                       port:                   5432,
                       user:                   String::from("hab"),
                       password:               None,
                       database:               String::from("builder"),
                       connection_retry_ms:    300,
                       connection_timeout_sec: 3600,
                       connection_test:        false,
                       pool_size:              (num_cpus::get() * 2) as u32,
                       ssl_mode:               None,
                       ssl_cert:               None,
                       ssl_key:                None,
                       ssl_root_cert:          None, }
    }
}

impl fmt::Display for DataStoreCfg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut connect = format!("postgres://{}", self.user);
        connect = match self.password {
            Some(ref p) => {
                // We can potentially get non-url friendly chars here so we need to encode them
                let encoded_password = utf8_percent_encode(p, PATH_SEGMENT_ENCODE_SET).to_string();
                format!("{}:{}", connect, encoded_password)
            }
            None => connect,
        };
        connect = format!("{}@{}:{}/{}", connect, self.host, self.port, self.database);
        let mut opts = Vec::new();

        if let Some(ref m) = self.ssl_mode {
            opts.push(format!("sslmode={}", m));
        }

        if let Some(ref m) = self.ssl_cert {
            opts.push(format!("sslcert={}", m));
        }

        if let Some(ref m) = self.ssl_key {
            opts.push(format!("sslkey={}", m));
        }

        if let Some(ref m) = self.ssl_root_cert {
            opts.push(format!("sslrootcert={}", m));
        }

        if !opts.is_empty() {
            connect = format!("{}?{}", connect, opts.join("&"));
        }

        write!(f, "{}", connect)
    }
}

impl<'a> IntoConnectParams for &'a DataStoreCfg {
    fn into_connect_params(self) -> Result<ConnectParams, Box<dyn Error + Sync + Send>> {
        let mut builder = ConnectParams::builder();
        builder.port(self.port);
        builder.user(&self.user, self.password.as_ref().map(|p| &**p));
        builder.database(&self.database);
        Ok(builder.build(Host::Tcp(self.host.to_string())))
    }
}
