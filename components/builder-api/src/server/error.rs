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

use actix_web::{self,
                http::StatusCode,
                HttpResponse,
                ResponseError};
use artifactory_client::error::ArtifactoryError;
use diesel;
use github_api_client::HubError;
use oauth_client::error::Error as OAuthError;
use protobuf;
use reqwest;
use rusoto_core::RusotoError;
use rusoto_s3;
use serde_json;
use std::{error,
          fmt,
          fs,
          io,
          result,
          string};

use crate::{bldr_core,
            db,
            hab_core};

#[derive(Debug)]
pub enum Error {
    Artifactory(ArtifactoryError),
    Authentication,
    Authorization,
    BadRequest,
    BuilderCore(bldr_core::Error),
    Conflict,
    CreateBucketError(RusotoError<rusoto_s3::CreateBucketError>),
    DbError(db::error::Error),
    DieselError(diesel::result::Error),
    Github(HubError),
    HabitatCore(hab_core::Error),
    HeadObject(RusotoError<rusoto_s3::HeadObjectError>),
    HttpClient(reqwest::Error),
    InnerError(io::IntoInnerError<io::BufWriter<fs::File>>),
    IO(io::Error),
    ListBuckets(RusotoError<rusoto_s3::ListBucketsError>),
    MultipartCompletion(RusotoError<rusoto_s3::CompleteMultipartUploadError>),
    MultipartUploadReq(RusotoError<rusoto_s3::CreateMultipartUploadError>),
    NotFound,
    OAuth(OAuthError),
    PackageDownload(RusotoError<rusoto_s3::GetObjectError>),
    PackageUpload(RusotoError<rusoto_s3::PutObjectError>),
    PartialUpload(RusotoError<rusoto_s3::UploadPartError>),
    PayloadError(actix_web::error::PayloadError),
    Protobuf(protobuf::ProtobufError),
    SerdeJson(serde_json::Error),
    System,
    TLSError(openssl::error::ErrorStack),
    Unprocessable,
    Utf8(string::FromUtf8Error),
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            Error::Artifactory(ref e) => format!("{}", e),
            Error::Authentication => "User is not authenticated".to_string(),
            Error::Authorization => "User is not authorized to perform operation".to_string(),
            Error::BadRequest => "Bad request".to_string(),
            Error::BuilderCore(ref e) => format!("{}", e),
            Error::Conflict => "Entity conflict".to_string(),
            Error::CreateBucketError(ref e) => format!("{}", e),
            Error::DbError(ref e) => format!("{}", e),
            Error::DieselError(ref e) => format!("{}", e),
            Error::Github(ref e) => format!("{}", e),
            Error::HabitatCore(ref e) => format!("{}", e),
            Error::HeadObject(ref e) => format!("{}", e),
            Error::HttpClient(ref e) => format!("{}", e),
            Error::InnerError(ref e) => format!("{}", e.error()),
            Error::IO(ref e) => format!("{}", e),
            Error::ListBuckets(ref e) => format!("{}", e),
            Error::MultipartCompletion(ref e) => format!("{}", e),
            Error::MultipartUploadReq(ref e) => format!("{}", e),
            Error::NotFound => "Entity not found".to_string(),
            Error::OAuth(ref e) => format!("{}", e),
            Error::PackageDownload(ref e) => format!("{}", e),
            Error::PackageUpload(ref e) => format!("{}", e),
            Error::PartialUpload(ref e) => format!("{}", e),
            Error::PayloadError(ref e) => format!("{}", e),
            Error::Protobuf(ref e) => format!("{}", e),
            Error::SerdeJson(ref e) => format!("{}", e),
            Error::System => "Internal error".to_string(),
            Error::TLSError(ref e) => format!("{}", e),
            Error::Unprocessable => "Unprocessable entity".to_string(),
            Error::Utf8(ref e) => format!("{}", e),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Artifactory(ref err) => err.description(),
            Error::Authentication => "User is not authenticated",
            Error::Authorization => "User is not authorized to perform operation",
            Error::BadRequest => "Http request formation error",
            Error::BuilderCore(ref err) => err.description(),
            Error::Conflict => "Entity conflict",
            Error::CreateBucketError(ref err) => err.description(),
            Error::DbError(ref err) => err.description(),
            Error::DieselError(ref err) => err.description(),
            Error::Github(ref err) => err.description(),
            Error::HabitatCore(ref err) => err.description(),
            Error::HeadObject(ref err) => err.description(),
            Error::HttpClient(ref err) => err.description(),
            Error::InnerError(ref err) => err.error().description(),
            Error::IO(ref err) => err.description(),
            Error::ListBuckets(ref err) => err.description(),
            Error::MultipartCompletion(ref err) => err.description(),
            Error::MultipartUploadReq(ref err) => err.description(),
            Error::NotFound => "Entity not found",
            Error::OAuth(ref err) => err.description(),
            Error::PackageDownload(ref err) => err.description(),
            Error::PackageUpload(ref err) => err.description(),
            Error::PartialUpload(ref err) => err.description(),
            Error::PayloadError(_) => "Http request stream error",
            Error::Protobuf(ref err) => err.description(),
            Error::SerdeJson(ref err) => err.description(),
            Error::System => "Internal error",
            Error::TLSError(ref err) => err.description(),
            Error::Unprocessable => "Unprocessable entity",
            Error::Utf8(ref err) => err.description(),
        }
    }
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match self {
            Error::Artifactory(ref e) => HttpResponse::new(artifactory_err_to_http(&e)),
            Error::Authentication => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Error::Authorization => HttpResponse::new(StatusCode::FORBIDDEN),
            Error::BadRequest => HttpResponse::new(StatusCode::BAD_REQUEST),
            Error::Conflict => HttpResponse::new(StatusCode::CONFLICT),
            Error::Github(_) => HttpResponse::new(StatusCode::FORBIDDEN),
            Error::NotFound => HttpResponse::new(StatusCode::NOT_FOUND),
            Error::OAuth(_) => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Error::DieselError(ref e) => HttpResponse::new(diesel_err_to_http(&e)),
            Error::System => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Error::Unprocessable => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),

            // Default
            _ => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
        }
    }
}

impl Into<HttpResponse> for Error {
    fn into(self) -> HttpResponse {
        match self {
            Error::Artifactory(ref e) => HttpResponse::new(artifactory_err_to_http(&e)),
            Error::Authentication => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Error::Authorization => HttpResponse::new(StatusCode::FORBIDDEN),
            Error::BadRequest => HttpResponse::new(StatusCode::BAD_REQUEST),
            Error::Conflict => HttpResponse::new(StatusCode::CONFLICT),
            Error::Github(_) => HttpResponse::new(StatusCode::FORBIDDEN),
            Error::NotFound => HttpResponse::new(StatusCode::NOT_FOUND),
            Error::OAuth(_) => HttpResponse::new(StatusCode::UNAUTHORIZED),
            Error::BuilderCore(ref e) => HttpResponse::new(bldr_core_err_to_http(e)),
            Error::DieselError(ref e) => HttpResponse::new(diesel_err_to_http(e)),
            Error::System => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Error::Unprocessable => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),

            // Default
            _ => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
        }
    }
}

fn artifactory_err_to_http(err: &ArtifactoryError) -> StatusCode {
    match err {
        ArtifactoryError::ApiError(code, _) => StatusCode::from_u16(code.as_u16()).unwrap(),
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn diesel_err_to_http(err: &diesel::result::Error) -> StatusCode {
    match err {
        diesel::result::Error::NotFound => StatusCode::NOT_FOUND,
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        ) => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn bldr_core_err_to_http(err: &bldr_core::Error) -> StatusCode {
    match err {
        bldr_core::error::Error::RpcError(code, _) => StatusCode::from_u16(*code).unwrap(),
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// From handlers - these make application level error handling cleaner

impl From<hab_core::Error> for Error {
    fn from(err: hab_core::Error) -> Error { Error::HabitatCore(err) }
}

impl From<bldr_core::Error> for Error {
    fn from(err: bldr_core::Error) -> Error { Error::BuilderCore(err) }
}

impl From<diesel::result::Error> for Error {
    fn from(err: diesel::result::Error) -> Error { Error::DieselError(err) }
}

impl From<HubError> for Error {
    fn from(err: HubError) -> Error { Error::Github(err) }
}

impl From<io::IntoInnerError<io::BufWriter<fs::File>>> for Error {
    fn from(err: io::IntoInnerError<io::BufWriter<fs::File>>) -> Error { Error::InnerError(err) }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self { Error::IO(err) }
}

impl From<OAuthError> for Error {
    fn from(err: OAuthError) -> Error { Error::OAuth(err) }
}

impl From<ArtifactoryError> for Error {
    fn from(err: ArtifactoryError) -> Error { Error::Artifactory(err) }
}

impl From<actix_web::error::PayloadError> for Error {
    fn from(err: actix_web::error::PayloadError) -> Error { Error::PayloadError(err) }
}

impl From<protobuf::ProtobufError> for Error {
    fn from(err: protobuf::ProtobufError) -> Error { Error::Protobuf(err) }
}

impl From<db::error::Error> for Error {
    fn from(err: db::error::Error) -> Error { Error::DbError(err) }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error { Error::SerdeJson(err) }
}

impl From<openssl::error::ErrorStack> for Error {
    fn from(err: openssl::error::ErrorStack) -> Error { Error::TLSError(err) }
}

impl From<string::FromUtf8Error> for Error {
    fn from(err: string::FromUtf8Error) -> Error { Error::Utf8(err) }
}

impl From<actix_web::error::BlockingError<std::io::Error>> for Error {
    fn from(err: actix_web::error::BlockingError<std::io::Error>) -> Error { err.into() }
}
