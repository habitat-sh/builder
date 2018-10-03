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
use std::fs;
use std::io;
use std::num;
use std::result;
use std::string;

use bldr_core;
use github_api_client::HubError;
use hab_core;
use hab_net::conn;
use hab_net::{self, ErrCode};
use oauth_client::error::Error as OAuthError;
use serde_json;

use actix;
use actix_web;
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
    ActixMailbox(actix::MailboxError),
    ActixWeb(actix_web::Error),
    Authentication,
    Authorization,
    CircularDependency(String),
    Connection(conn::ConnErr),
    BadRequest(String),
    Github(HubError),
    InnerError(io::IntoInnerError<io::BufWriter<fs::File>>),
    Protocol(protocol::ProtocolError),
    HabitatCore(hab_core::Error),
    IO(io::Error),
    NetError(hab_net::NetError),
    ParseIntError(num::ParseIntError),
    PayloadError(actix_web::error::PayloadError),
    Protobuf(protobuf::ProtobufError),
    UnknownGitHubEvent(String),
    Zmq(zmq::Error),
    CreateBucketError(rusoto_s3::CreateBucketError),
    BuilderCore(bldr_core::Error),
    HeadObject(rusoto_s3::HeadObjectError),
    ListBuckets(rusoto_s3::ListBucketsError),
    MultipartCompletion(rusoto_s3::CompleteMultipartUploadError),
    MultipartUploadReq(rusoto_s3::CreateMultipartUploadError),
    OAuth(OAuthError),
    PackageUpload(rusoto_s3::PutObjectError),
    PackageDownload(rusoto_s3::GetObjectError),
    PartialUpload(rusoto_s3::UploadPartError),
    SerdeJson(serde_json::Error),
    UnsupportedPlatform(String),
    Utf8(string::FromUtf8Error),
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            Error::ActixMailbox(ref e) => format!("{}", e),
            Error::ActixWeb(ref e) => format!("{}", e),
            Error::Authentication => "User is not authenticated".to_string(),
            Error::Authorization => "User is not authorized to perform operation".to_string(),
            Error::BadRequest(ref e) => format!("{}", e),
            Error::CircularDependency(ref e) => {
                format!("Circular dependency detected for package upload: {}", e)
            }
            Error::Connection(ref e) => format!("{}", e),
            Error::Github(ref e) => format!("{}", e),
            Error::InnerError(ref e) => format!("{}", e.error()),
            Error::ParseIntError(ref e) => format!("{}", e),
            Error::PayloadError(ref e) => format!("{}", e),
            Error::Protocol(ref e) => format!("{}", e),
            Error::HabitatCore(ref e) => format!("{}", e),
            Error::IO(ref e) => format!("{}", e),
            Error::NetError(ref e) => format!("{}", e),
            Error::OAuth(ref e) => format!("{}", e),
            Error::Protobuf(ref e) => format!("{}", e),
            Error::UnknownGitHubEvent(ref e) => {
                format!("Unknown or unsupported GitHub event, {}", e)
            }
            Error::Zmq(ref e) => format!("{}", e),
            Error::CreateBucketError(ref e) => format!("{}", e),
            Error::BuilderCore(ref e) => format!("{}", e),
            Error::HeadObject(ref e) => format!("{}", e),
            Error::ListBuckets(ref e) => format!("{}", e),
            Error::MultipartCompletion(ref e) => format!("{}", e),
            Error::MultipartUploadReq(ref e) => format!("{}", e),
            Error::PackageUpload(ref e) => format!("{}", e),
            Error::PackageDownload(ref e) => format!("{}", e),
            Error::PartialUpload(ref e) => format!("{}", e),
            Error::SerdeJson(ref e) => format!("{}", e),
            Error::UnsupportedPlatform(ref e) => {
                format!("Unsupported platform or architecture: {}", e)
            }
            Error::Utf8(ref e) => format!("{}", e),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::ActixMailbox(_) => "Mailbox is ded",
            Error::ActixWeb(_) => "Web is ded",
            Error::Authentication => "User is not authenticated",
            Error::Authorization => "User is not authorized to perform operation",
            Error::BadRequest(_) => "Http request formation error",
            Error::CircularDependency(_) => "Circular dependency detected for package upload",
            Error::Connection(ref err) => err.description(),
            Error::Github(ref err) => err.description(),
            Error::InnerError(ref err) => err.error().description(),
            Error::ParseIntError(ref err) => err.description(),
            Error::PayloadError(_) => "Http request stream error",
            Error::Protocol(ref err) => err.description(),
            Error::HabitatCore(ref err) => err.description(),
            Error::IO(ref err) => err.description(),
            Error::NetError(ref err) => err.description(),
            Error::OAuth(ref err) => err.description(),
            Error::Protobuf(ref err) => err.description(),
            Error::UnknownGitHubEvent(_) => {
                "Unknown or unsupported GitHub event received in request"
            }
            Error::Zmq(ref err) => err.description(),
            Error::CreateBucketError(ref err) => err.description(),
            Error::BuilderCore(ref err) => err.description(),
            Error::HeadObject(ref err) => err.description(),
            Error::ListBuckets(ref err) => err.description(),
            Error::MultipartCompletion(ref err) => err.description(),
            Error::MultipartUploadReq(ref err) => err.description(),
            Error::PackageUpload(ref err) => err.description(),
            Error::PackageDownload(ref err) => err.description(),
            Error::PartialUpload(ref err) => err.description(),
            Error::SerdeJson(ref err) => err.description(),
            Error::UnsupportedPlatform(_) => "Unsupported platform or architecture",
            Error::Utf8(ref err) => err.description(),
        }
    }
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match self {
            Error::Authentication => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Error::Authorization => HttpResponse::new(StatusCode::FORBIDDEN),
            Error::BadRequest(ref err) => {
                HttpResponse::with_body(StatusCode::BAD_REQUEST, err.to_owned())
            }
            Error::Github(_) => HttpResponse::new(StatusCode::FORBIDDEN),
            Error::CircularDependency(_) => HttpResponse::new(StatusCode::FAILED_DEPENDENCY),
            Error::NetError(ref e) => HttpResponse::new(net_err_to_http(&e)),
            Error::OAuth(_) => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Error::ParseIntError(_) => HttpResponse::new(StatusCode::BAD_REQUEST),
            Error::Protocol(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),

            // Default
            _ => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
        }
    }
}

impl Into<HttpResponse> for Error {
    fn into(self) -> HttpResponse {
        match self {
            Error::Authentication => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Error::Authorization => HttpResponse::new(StatusCode::FORBIDDEN),
            Error::BadRequest(_) => HttpResponse::new(StatusCode::BAD_REQUEST),
            Error::Github(_) => HttpResponse::new(StatusCode::FORBIDDEN),
            Error::CircularDependency(_) => HttpResponse::new(StatusCode::FAILED_DEPENDENCY),
            Error::NetError(ref e) => HttpResponse::new(net_err_to_http(&e)),
            Error::OAuth(_) => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Error::ParseIntError(_) => HttpResponse::new(StatusCode::BAD_REQUEST),
            Error::Protocol(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),

            // Default
            _ => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
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

        ErrCode::GROUP_NOT_COMPLETE
        | ErrCode::BUILD
        | ErrCode::EXPORT
        | ErrCode::POST_PROCESSOR
        | ErrCode::SECRET_KEY_IMPORT
        | ErrCode::INVALID_INTEGRATIONS => StatusCode::UNPROCESSABLE_ENTITY,

        ErrCode::PARTIAL_JOB_GROUP_PROMOTE => StatusCode::PARTIAL_CONTENT,

        ErrCode::BUG
        | ErrCode::SYS
        | ErrCode::DATA_STORE
        | ErrCode::WORKSPACE_SETUP
        | ErrCode::REG_CONFLICT
        | ErrCode::REG_NOT_FOUND => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// From handlers - these make application level error handling cleaner

impl From<actix::MailboxError> for Error {
    fn from(err: actix::MailboxError) -> Error {
        Error::ActixMailbox(err)
    }
}

impl From<actix_web::Error> for Error {
    fn from(err: actix_web::Error) -> Error {
        Error::ActixWeb(err)
    }
}

impl From<hab_core::Error> for Error {
    fn from(err: hab_core::Error) -> Error {
        Error::HabitatCore(err)
    }
}

impl From<bldr_core::Error> for Error {
    fn from(err: bldr_core::Error) -> Error {
        Error::BuilderCore(err)
    }
}

impl From<HubError> for Error {
    fn from(err: HubError) -> Error {
        Error::Github(err)
    }
}

impl From<io::IntoInnerError<io::BufWriter<fs::File>>> for Error {
    fn from(err: io::IntoInnerError<io::BufWriter<fs::File>>) -> Error {
        Error::InnerError(err)
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

impl From<num::ParseIntError> for Error {
    fn from(err: num::ParseIntError) -> Self {
        Error::ParseIntError(err)
    }
}
impl From<OAuthError> for Error {
    fn from(err: OAuthError) -> Error {
        Error::OAuth(err)
    }
}

impl From<actix_web::error::PayloadError> for Error {
    fn from(err: actix_web::error::PayloadError) -> Error {
        Error::PayloadError(err)
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

impl From<string::FromUtf8Error> for Error {
    fn from(err: string::FromUtf8Error) -> Error {
        Error::Utf8(err)
    }
}

impl From<zmq::Error> for Error {
    fn from(err: zmq::Error) -> Error {
        Error::Zmq(err)
    }
}
