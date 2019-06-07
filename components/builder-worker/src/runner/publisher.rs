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

use retry::{delay::Fixed,
            retry};

use crate::{bldr_core::{api_client::ApiClient,
                        logger::Logger},
            hab_core::{package::archive::PackageArchive,
                       ChannelIdent}};

use super::{RETRIES,
            RETRY_WAIT};
use crate::error::{Error,
                   Result};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Publisher {
    pub enabled:     bool,
    pub url:         String,
    pub channel_opt: Option<ChannelIdent>,
}

impl Publisher {
    pub fn run(&mut self,
               archive: &mut PackageArchive,
               auth_token: &str,
               logger: &mut Logger)
               -> Result<()> {
        if !self.enabled {
            debug!("Publishing skipped (not enabled)");
            return Ok(());
        }
        debug!("Publisher (url: {}, channel: {:?})",
               self.url, self.channel_opt);

        let client = ApiClient::new(&self.url);
        let ident = archive.ident().unwrap();
        let target = archive.target().unwrap();

        match retry(Fixed::from_millis(RETRY_WAIT).take(RETRIES), || {
                  // let res = client.x_put_package(archive, auth_token);
                  match client.x_put_package(archive, auth_token) {
                      Ok(_) => Ok(()),
                      Err(err) => {
                          let msg = format!("Upload {}: {:?}", ident, err);
                          debug!("{}", msg);
                          logger.log(&msg);
                          Err(err)
                      }
                  }
              }) {
            Ok(_) => (),
            Err(err) => {
                let msg = format!("Failed to upload {} after {} retries", ident, RETRIES);
                warn!("{}", msg);
                logger.log(&msg);
                return Err(Error::Retry(err));
            }
        }

        if let Some(channel) = &self.channel_opt {
            if channel != &ChannelIdent::stable() && channel != &ChannelIdent::unstable() {
                match retry(Fixed::from_millis(RETRY_WAIT).take(RETRIES), || {
                          let res = client.create_channel(&ident.origin, &channel, auth_token);
                          match res {
                              Ok(_) => Ok(()),
                              Err(err) => {
                                  let msg = format!("Create channel {}: {:?}", channel, err);
                                  debug!("{}", msg);
                                  logger.log(&msg);
                                  Err(err)
                              }
                          }
                      }) {
                    Ok(_) => (),
                    Err(err) => {
                        let msg = format!("Failed to create channel {} after {} retries",
                                          channel, RETRIES);
                        warn!("{}", msg);
                        logger.log(&msg);
                        return Err(Error::Retry(err));
                    }
                }
            }

            match retry(Fixed::from_millis(RETRY_WAIT).take(RETRIES), || {
                      let res = client.promote_package((&ident, target), channel, auth_token);
                      if res.is_err() {
                          let msg = format!("Promote {} to {}: {:?}", ident, channel, res);
                          debug!("{}", msg);
                          logger.log(&msg);
                      };
                      res
                  }) {
                Ok(_) => (),
                Err(err) => {
                    let msg = format!("Failed to promote {} to {} after {} retries",
                                      ident, channel, RETRIES);
                    warn!("{}", msg);
                    logger.log(&msg);
                    return Err(Error::Retry(err));
                }
            }
        } else {
            debug!("Promotion skipped (no channel specified)");
        }
        Ok(())
    }
}
