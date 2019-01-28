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

use retry::retry;

use crate::bldr_core::api_client::ApiClient;
use crate::bldr_core::logger::Logger;
use crate::hab_core::package::archive::PackageArchive;
use crate::hab_core::ChannelIdent;

use super::{RETRIES, RETRY_WAIT};
use crate::error::{Error, Result};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Publisher {
    pub enabled: bool,
    pub url: String,
    pub channel_opt: Option<ChannelIdent>,
}

impl Publisher {
    pub fn run(
        &mut self,
        archive: &mut PackageArchive,
        auth_token: &str,
        logger: &mut Logger,
    ) -> Result<()> {
        if !self.enabled {
            debug!("Publishing skipped (not enabled)");
            return Ok(());
        }
        debug!(
            "Publisher (url: {}, channel: {:?})",
            self.url, self.channel_opt
        );

        let client = ApiClient::new(&self.url);
        let ident = archive.ident().unwrap();

        match retry(
            RETRIES,
            RETRY_WAIT,
            || client.x_put_package(archive, auth_token),
            |res| match *res {
                Ok(_) => true,
                Err(_) => {
                    let msg = format!("Upload {}: {:?}", ident, res);
                    debug!("{}", msg);
                    logger.log(&msg);
                    false
                }
            },
        ) {
            Ok(_) => (),
            Err(err) => {
                let msg = format!("Failed to upload {} after {} retries", ident, RETRIES);
                warn!("{}", msg);
                logger.log(&msg);
                return Err(Error::Retry(err));
            }
        }

        if let Some(channel) = &self.channel_opt {
            let channel = ChannelIdent::from(channel.clone());

            if channel != ChannelIdent::stable() && channel != ChannelIdent::unstable() {
                match retry(
                    RETRIES,
                    RETRY_WAIT,
                    || client.create_channel(&ident.origin, &channel.to_string(), auth_token),
                    |res| match *res {
                        Ok(_) => true,
                        Err(_) => {
                            let msg = format!("Create channel {}: {:?}", channel, res);
                            debug!("{}", msg);
                            logger.log(&msg);
                            false
                        }
                    },
                ) {
                    Ok(_) => (),
                    Err(err) => {
                        let msg = format!(
                            "Failed to create channel {} after {} retries",
                            channel, RETRIES
                        );
                        warn!("{}", msg);
                        logger.log(&msg);
                        return Err(Error::Retry(err));
                    }
                }
            }

            match retry(
                RETRIES,
                RETRY_WAIT,
                || client.promote_package(&ident, &channel.to_string(), auth_token),
                |res| {
                    if res.is_err() {
                        let msg = format!("Promote {} to {}: {:?}", ident, channel, res);
                        debug!("{}", msg);
                        logger.log(&msg);
                    };
                    res.is_ok()
                },
            ) {
                Ok(_) => (),
                Err(err) => {
                    let msg = format!(
                        "Failed to promote {} to {} after {} retries",
                        ident, channel, RETRIES
                    );
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
