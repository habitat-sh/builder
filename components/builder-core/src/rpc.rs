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

use std::{io::Read,
          iter::FromIterator};

use reqwest::{blocking::Client,
              header::HeaderMap,
              StatusCode};

use protobuf;
use serde_json;

use crate::{error::{Error,
                    Result},
            http_client::{ACCEPT_APPLICATION_JSON,
                          CONTENT_TYPE_APPLICATION_JSON,
                          USER_AGENT_BLDR}};

// RPC message, transport as JSON over HTTP
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RpcMessage {
    #[serde(default)]
    pub id:   String,
    #[serde(default)]
    pub body: Vec<u8>,
}

impl RpcMessage {
    pub fn new(id: String, body: Vec<u8>) -> Self { RpcMessage { id, body } }

    pub fn make<T>(msg: &T) -> Result<RpcMessage>
        where T: protobuf::Message
    {
        let id = msg.descriptor().name().to_owned();
        let body = msg.write_to_bytes().map_err(Error::Protobuf)?;

        Ok(RpcMessage::new(id, body))
    }

    pub fn parse<T>(&self) -> Result<T>
        where T: protobuf::Message
    {
        protobuf::parse_from_bytes::<T>(&self.body).map_err(Error::Protobuf)
    }
}

// RPC client
pub struct RpcClient {
    cli:      Client,
    endpoint: String,
}

impl RpcClient {
    pub fn new(url: &str) -> Self {
        debug!("Creating RPC client, url = {}", url);

        let header_values = vec![USER_AGENT_BLDR.clone(),
                                 ACCEPT_APPLICATION_JSON.clone(),
                                 CONTENT_TYPE_APPLICATION_JSON.clone()];
        let headers = HeaderMap::from_iter(header_values.into_iter());

        let cli = match Client::builder().default_headers(headers).build() {
            Ok(client) => client,
            Err(err) => panic!("Unable to create Rpc client, err = {}", err),
        };

        RpcClient { cli,
                    endpoint: format!("{}/rpc", url) }
    }

    pub fn rpc<R, T>(&self, req: &R) -> Result<T>
        where R: protobuf::Message,
              T: protobuf::Message
    {
        let id = req.descriptor().name().to_owned();
        let body = req.write_to_bytes()?;
        let msg = RpcMessage { id, body };
        debug!("Sending RPC Message: {}", msg.id);

        let json = serde_json::to_string(&msg)?;
        let mut res = match self.cli.post(&self.endpoint).body(json).send() {
            Ok(res) => res,
            Err(err) => {
                debug!("Got http error: {}", err);
                return Err(Error::HttpClient(err));
            }
        };
        debug!("Got RPC response status: {}", res.status());

        let mut s = String::new();
        res.read_to_string(&mut s).map_err(Error::IO)?;
        trace!("Got http response body: {}", s);

        match res.status() {
            StatusCode::OK => {
                let resp_json: RpcMessage = serde_json::from_str(&s)?;
                trace!("Got RPC JSON: {:?}", resp_json);

                let resp_msg = protobuf::parse_from_bytes::<T>(&resp_json.body)?;
                Ok(resp_msg)
            }
            status => Err(Error::RpcError(status.as_u16(), s)),
        }
    }
}
