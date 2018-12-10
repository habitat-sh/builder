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

use std::error;
use std::fmt;
use std::result;
use std::string::FromUtf8Error;

use protobuf;

#[derive(Debug)]
pub enum ProtocolError {
    BadJobGroupProjectState(String),
    BadJobGroupState(String),
    BadJobState(String),
    BadOriginPackageVisibility(String),
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

impl error::Error for ProtocolError {
    fn description(&self) -> &str {
        match *self {
            ProtocolError::BadJobGroupProjectState(_) => "Job Group Project state cannot be parsed",
            ProtocolError::BadJobGroupState(_) => "Job Group state cannot be parsed",
            ProtocolError::BadJobState(_) => "Job state cannot be parsed",
            ProtocolError::BadOriginPackageVisibility(_) => {
                "Origin package visibility cannot be parsed"
            }
            ProtocolError::Decode(_) => "Unable to decode protocol message",
            ProtocolError::Encode(_) => "Unable to encode protocol message",
            ProtocolError::IdentityDecode(_) => "Unable to decode identity message part",
            ProtocolError::NoProtocol(_) => "No `net::Protocol` matches the given string",
        }
    }
}
