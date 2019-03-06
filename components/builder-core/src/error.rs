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

use std::{error,
          fmt,
          io,
          result,
          string};

use base64;
use chrono;
use protobuf;
use reqwest;
use serde_json;

use crate::{hab_core,
            protocol};

#[derive(Debug)]
pub enum Error {
    ApiError(reqwest::StatusCode, String),
    RpcError(u16, String),
    HttpClient(reqwest::Error),
    IO(io::Error),
    Base64Error(base64::DecodeError),
    ChronoError(chrono::format::ParseError),
    DecryptError(String),
    EncryptError(String),
    FromUtf8Error(string::FromUtf8Error),
    HabitatCore(hab_core::Error),
    Protobuf(protobuf::ProtobufError),
    Protocol(protocol::ProtocolError),
    Serialization(serde_json::Error),
    TokenInvalid,
    TokenExpired,
    BadResponse,
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            Error::ApiError(ref code, ref response) => {
                format!("Received a non-200 response, status={}, response={:?}",
                        code, response)
            }
            Error::RpcError(ref code, ref e) => format!("{} {}", code, e),
            Error::HttpClient(ref e) => format!("{}", e),
            Error::IO(ref e) => format!("{}", e),
            Error::Base64Error(ref e) => format!("{}", e),
            Error::ChronoError(ref e) => format!("{}", e),
            Error::DecryptError(ref e) => e.to_string(),
            Error::EncryptError(ref e) => e.to_string(),
            Error::FromUtf8Error(ref e) => format!("{}", e),
            Error::HabitatCore(ref e) => format!("{}", e),
            Error::Protobuf(ref e) => format!("{}", e),
            Error::Protocol(ref e) => format!("{}", e),
            Error::Serialization(ref e) => format!("{}", e),
            Error::TokenInvalid => "Token is invalid".to_string(),
            Error::TokenExpired => "Token is expired".to_string(),
            Error::BadResponse => "Response missing required fields".to_string(),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::ApiError(..) => "Response returned a non-200 status code.",
            Error::RpcError(..) => "Response returned a non-200 status code.",
            Error::HttpClient(ref err) => err.description(),
            Error::IO(ref err) => err.description(),
            Error::Base64Error(ref e) => e.description(),
            Error::ChronoError(ref e) => e.description(),
            Error::DecryptError(_) => "Error decrypting integration",
            Error::EncryptError(_) => "Error encrypting integration",
            Error::FromUtf8Error(ref e) => e.description(),
            Error::HabitatCore(ref err) => err.description(),
            Error::Protobuf(ref err) => err.description(),
            Error::Protocol(ref err) => err.description(),
            Error::Serialization(ref err) => err.description(),
            Error::TokenInvalid => "Token is invalid",
            Error::TokenExpired => "Token is expired",
            Error::BadResponse => "Response missing required fields",
        }
    }
}

impl From<hab_core::Error> for Error {
    fn from(err: hab_core::Error) -> Error { Error::HabitatCore(err) }
}

impl From<protobuf::ProtobufError> for Error {
    fn from(err: protobuf::ProtobufError) -> Error { Error::Protobuf(err) }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Error { Error::HttpClient(err) }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error { Error::Serialization(err) }
}
