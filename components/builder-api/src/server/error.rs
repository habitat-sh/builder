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
use std::ffi;
use std::fmt;
use std::io;
use std::result;

use bldr_core;
use github_api_client::HubError;
use hab_core;
use hab_core::package::{self, Identifiable};
use hab_net::conn;
use hab_net::{self, ErrCode};
use oauth_client::error::Error as OAuthError;
use serde_json;

use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use protobuf;
use protocol;
use rusoto_s3;
use zmq;

// TODO: We've probably gone overboard with the number of errors we
// are wrapping - review whether we need more than one error per module
#[derive(Debug)]
pub enum Error {
    Authorization(String),
    Connection(conn::ConnErr),
    Github(HubError),
    Protocol(protocol::ProtocolError),
    BadPort(String),
    HabitatCore(hab_core::Error),
    IO(io::Error),
    NetError(hab_net::NetError),
    Protobuf(protobuf::ProtobufError),
    UnknownGitHubEvent(String),
    Zmq(zmq::Error),
    ChannelAlreadyExists(String),
    ChannelDoesNotExist(String),
    CreateBucketError(rusoto_s3::CreateBucketError),
    DepotClientError(bldr_core::Error),
    HabitatNet(hab_net::error::LibError),
    HeadObject(rusoto_s3::HeadObjectError),
    InvalidPackageIdent(String),
    ListBuckets(rusoto_s3::ListBucketsError),
    ListObjects(rusoto_s3::ListObjectsError),
    MessageTypeNotFound,
    MultipartCompletion(rusoto_s3::CompleteMultipartUploadError),
    MultipartUploadReq(rusoto_s3::CreateMultipartUploadError),
    NoXFilename,
    NoFilePart,
    NulError(ffi::NulError),
    OAuth(OAuthError),
    ObjectError(rusoto_s3::ListObjectsError),
    PackageIsAlreadyInChannel(String, String),
    PackageUpload(rusoto_s3::PutObjectError),
    PackageDownload(rusoto_s3::GetObjectError),
    PartialUpload(rusoto_s3::UploadPartError),
    RemotePackageNotFound(package::PackageIdent),
    SerdeJson(serde_json::Error),
    UnsupportedPlatform(String),
    WriteSyncFailed,
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            Error::Authorization(ref e) => format!("Not authorized: {}", e),
            Error::Connection(ref e) => format!("{}", e),
            Error::Github(ref e) => format!("{}", e),
            Error::Protocol(ref e) => format!("{}", e),
            Error::BadPort(ref e) => format!("{} is an invalid port. Valid range 1-65535.", e),
            Error::HabitatCore(ref e) => format!("{}", e),
            Error::IO(ref e) => format!("{}", e),
            Error::NetError(ref e) => format!("{}", e),
            Error::OAuth(ref e) => format!("{}", e),
            Error::Protobuf(ref e) => format!("{}", e),
            Error::UnknownGitHubEvent(ref e) => {
                format!("Unknown or unsupported GitHub event, {}", e)
            }
            Error::Zmq(ref e) => format!("{}", e),
            Error::ChannelAlreadyExists(ref e) => format!("{} already exists.", e),
            Error::ChannelDoesNotExist(ref e) => format!("{} does not exist.", e),
            Error::CreateBucketError(ref e) => format!("{}", e),
            Error::DepotClientError(ref e) => format!("{}", e),
            Error::HabitatNet(ref e) => format!("{}", e),
            Error::HeadObject(ref e) => format!("{}", e),
            Error::InvalidPackageIdent(ref e) => format!(
                "Invalid package identifier: {:?}. A valid identifier is in the form \
                 origin/name (example: acme/redis)",
                e
            ),
            Error::ListBuckets(ref e) => format!("{}", e),
            Error::ListObjects(ref e) => format!("{}", e),
            Error::MessageTypeNotFound => format!("Unable to find message for given type"),
            Error::MultipartCompletion(ref e) => format!("{}", e),
            Error::MultipartUploadReq(ref e) => format!("{}", e),
            Error::NoXFilename => {
                format!("Invalid download from Builder - missing X-Filename header")
            }
            Error::NoFilePart => format!(
                "An invalid path was passed - we needed a filename, and this path does \
                 not have one"
            ),
            Error::NulError(ref e) => format!("{}", e),
            Error::ObjectError(ref e) => format!("{}", e),
            Error::PackageIsAlreadyInChannel(ref p, ref c) => {
                format!("{} is already in the {} channel.", p, c)
            }
            Error::PackageUpload(ref e) => format!("{}", e),
            Error::PackageDownload(ref e) => format!("{}", e),
            Error::PartialUpload(ref e) => format!("{}", e),
            Error::RemotePackageNotFound(ref pkg) => {
                if pkg.fully_qualified() {
                    format!("Cannot find package in any sources: {}", pkg)
                } else {
                    format!("Cannot find a release of package in any sources: {}", pkg)
                }
            }
            Error::SerdeJson(ref e) => format!("{}", e),
            Error::UnsupportedPlatform(ref e) => {
                format!("Unsupported platform or architecture: {}", e)
            }
            Error::WriteSyncFailed => {
                format!("Could not write to destination; perhaps the disk is full?")
            }
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Authorization(_) => "User is not authorized to perform operation",
            Error::Connection(ref err) => err.description(),
            Error::Github(ref err) => err.description(),
            Error::Protocol(ref err) => err.description(),
            Error::BadPort(_) => "Received an invalid port or a number outside of the valid range.",
            Error::HabitatCore(ref err) => err.description(),
            Error::IO(ref err) => err.description(),
            Error::NetError(ref err) => err.description(),
            Error::OAuth(ref err) => err.description(),
            Error::Protobuf(ref err) => err.description(),
            Error::UnknownGitHubEvent(_) => {
                "Unknown or unsupported GitHub event received in request"
            }
            Error::Zmq(ref err) => err.description(),
            Error::ChannelAlreadyExists(_) => "Channel already exists.",
            Error::ChannelDoesNotExist(_) => "Channel does not exist.",
            Error::CreateBucketError(ref err) => err.description(),
            Error::DepotClientError(ref err) => err.description(),
            Error::HabitatNet(ref err) => err.description(),
            Error::HeadObject(ref err) => err.description(),
            Error::InvalidPackageIdent(_) => {
                "Package identifiers must be in origin/name format (example: acme/redis)"
            }
            Error::ListBuckets(ref err) => err.description(),
            Error::ListObjects(ref err) => err.description(),
            Error::MultipartCompletion(ref err) => err.description(),
            Error::MultipartUploadReq(ref err) => err.description(),
            Error::NulError(_) => {
                "An attempt was made to build a CString with a null byte inside it"
            }
            Error::ObjectError(ref err) => err.description(),
            Error::PackageIsAlreadyInChannel(_, _) => "Package is already in channel",
            Error::PackageUpload(ref err) => err.description(),
            Error::PackageDownload(ref err) => err.description(),
            Error::PartialUpload(ref err) => err.description(),
            Error::RemotePackageNotFound(_) => "Cannot find a package in any sources",
            Error::NoXFilename => "Invalid download from Builder - missing X-Filename header",
            Error::NoFilePart => {
                "An invalid path was passed - we needed a filename, and this path does not have one"
            }
            Error::MessageTypeNotFound => "Unable to find message for given type",
            Error::SerdeJson(ref err) => err.description(),
            Error::UnsupportedPlatform(_) => "Unsupported platform or architecture",
            Error::WriteSyncFailed => {
                "Could not write to destination; bytes written was 0 on a non-0 buffer"
            }
        }
    }
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match self {
            Error::Authorization(_) => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Error::NetError(ref e) => HttpResponse::new(net_err_to_http(&e)),
            Error::OAuth(_) => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Error::Protocol(_) => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
            Error::SerdeJson(_) => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
            // TODO : Tackle the others...
            _ => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

impl Into<HttpResponse> for Error {
    fn into(self) -> HttpResponse {
        match self {
            Error::Authorization(_) => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Error::NetError(ref e) => HttpResponse::new(net_err_to_http(&e)),
            Error::OAuth(_) => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Error::Protocol(_) => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
            Error::SerdeJson(_) => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
            // TODO : Tackle the others...
            _ => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

fn net_err_to_http(err: &hab_net::NetError) -> StatusCode {
    match err.code() {
        ErrCode::TIMEOUT => StatusCode::GATEWAY_TIMEOUT,
        ErrCode::REMOTE_REJECTED => StatusCode::NOT_ACCEPTABLE,
        ErrCode::ENTITY_NOT_FOUND => StatusCode::NOT_FOUND,
        ErrCode::ENTITY_CONFLICT => StatusCode::CONFLICT,

        ErrCode::ACCESS_DENIED | ErrCode::SESSION_EXPIRED => StatusCode::UNAUTHORIZED,

        ErrCode::BAD_REMOTE_REPLY | ErrCode::SECRET_KEY_FETCH | ErrCode::VCS_CLONE => {
            StatusCode::BAD_GATEWAY
        }

        ErrCode::NO_SHARD | ErrCode::SOCK | ErrCode::REMOTE_UNAVAILABLE => {
            StatusCode::SERVICE_UNAVAILABLE
        }

        ErrCode::BAD_TOKEN => StatusCode::FORBIDDEN,
        ErrCode::GROUP_NOT_COMPLETE => StatusCode::UNPROCESSABLE_ENTITY,
        ErrCode::PARTIAL_JOB_GROUP_PROMOTE => StatusCode::PARTIAL_CONTENT,

        ErrCode::BUG
        | ErrCode::POST_PROCESSOR
        | ErrCode::BUILD
        | ErrCode::EXPORT
        | ErrCode::SYS
        | ErrCode::DATA_STORE
        | ErrCode::WORKSPACE_SETUP
        | ErrCode::SECRET_KEY_IMPORT
        | ErrCode::INVALID_INTEGRATIONS
        | ErrCode::REG_CONFLICT
        | ErrCode::REG_NOT_FOUND => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// From handlers - these make application level error handling cleaner
// TODO :Moving forward, leverage these instead of map_errs all over the place

impl From<hab_core::Error> for Error {
    fn from(err: hab_core::Error) -> Error {
        Error::HabitatCore(err)
    }
}

impl From<HubError> for Error {
    fn from(err: HubError) -> Error {
        Error::Github(err)
    }
}

impl From<hab_net::NetError> for Error {
    fn from(err: hab_net::NetError) -> Self {
        Error::NetError(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IO(err)
    }
}

impl From<OAuthError> for Error {
    fn from(err: OAuthError) -> Error {
        Error::OAuth(err)
    }
}

impl From<protobuf::ProtobufError> for Error {
    fn from(err: protobuf::ProtobufError) -> Error {
        Error::Protobuf(err)
    }
}

impl From<protocol::ProtocolError> for Error {
    fn from(err: protocol::ProtocolError) -> Error {
        Error::Protocol(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::SerdeJson(err)
    }
}

impl From<zmq::Error> for Error {
    fn from(err: zmq::Error) -> Error {
        Error::Zmq(err)
    }
}
