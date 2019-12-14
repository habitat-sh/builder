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
          result};

use crate::{db,
            hab_core};

use postgres;
use protobuf;
use r2d2;

#[derive(Debug)]
pub enum Error {
    Db(db::error::Error),
    DbPoolTimeout(r2d2::Error),
    DieselError(diesel::result::Error),
    DbTransaction(postgres::error::Error),
    HabitatCore(hab_core::Error),
    IO(io::Error),
    JobGraphPackagesGet(postgres::error::Error),
    Protobuf(protobuf::ProtobufError),
    UnknownJobGraphPackage,
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            Error::Db(ref e) => format!("{}", e),
            Error::DbPoolTimeout(ref e) => {
                format!("Timeout getting connection from the database pool, {}", e)
            }
            Error::DbTransaction(ref e) => format!("Database transaction error, {}", e),
            Error::DieselError(ref e) => format!("Diesel error, {}", e),
            Error::HabitatCore(ref e) => format!("{}", e),
            Error::IO(ref e) => format!("{}", e),
            Error::JobGraphPackagesGet(ref e) => {
                format!("Database error retrieving packages, {}", e)
            }
            Error::Protobuf(ref e) => format!("{}", e),
            Error::UnknownJobGraphPackage => "Unknown Package".to_string(),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Db(ref err) => err.description(),
            Error::DbPoolTimeout(ref err) => err.description(),
            Error::DbTransaction(ref err) => err.description(),
            Error::DieselError(ref err) => err.description(),
            Error::HabitatCore(ref err) => err.description(),
            Error::IO(ref err) => err.description(),
            Error::JobGraphPackagesGet(ref err) => err.description(),
            Error::Protobuf(ref err) => err.description(),
            Error::UnknownJobGraphPackage => "Unknown Package",
        }
    }
}

impl From<hab_core::Error> for Error {
    fn from(err: hab_core::Error) -> Error { Error::HabitatCore(err) }
}

impl From<db::error::Error> for Error {
    fn from(err: db::error::Error) -> Self { Error::Db(err) }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error { Error::IO(err) }
}

impl From<protobuf::ProtobufError> for Error {
    fn from(err: protobuf::ProtobufError) -> Error { Error::Protobuf(err) }
}
