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

use std::{fmt,
          result,
          string::FromUtf8Error};

#[derive(Debug)]
pub enum ProtocolError {
    BadJobGroupProjectState(String),
    BadJobGroupState(String),
    BadJobState(String),
    BadOriginPackageVisibility(String),
    BadOs(String),
    Decode(protobuf::ProtobufError),
    Encode(protobuf::ProtobufError),
    IdentityDecode(FromUtf8Error),
    NoProtocol(String),
}

pub type ProtocolResult<T> = result::Result<T, ProtocolError>;

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            ProtocolError::BadJobGroupProjectState(ref e) => {
                format!("Bad Job Group Project State {}", e)
            }
            ProtocolError::BadJobGroupState(ref e) => format!("Bad Job Group State {}", e),
            ProtocolError::BadJobState(ref e) => format!("Bad Job State {}", e),
            ProtocolError::BadOriginPackageVisibility(ref e) => {
                format!("Bad Origin Package Visibility {}", e)
            }
            ProtocolError::BadOs(ref e) => format!("Bad OS {}", e),
            ProtocolError::Decode(ref e) => format!("Unable to decode protocol message, {}", e),
            ProtocolError::Encode(ref e) => format!("Unable to encode protocol message, {}", e),
            ProtocolError::IdentityDecode(ref e) => {
                format!("Unable to decode identity message part, {}", e)
            }
            ProtocolError::NoProtocol(ref e) => {
                format!("No `net::Protocol` matching given string, {}", e)
            }
        };
        write!(f, "{}", msg)
    }
}
