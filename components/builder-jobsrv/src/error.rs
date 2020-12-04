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
          num,
          path::PathBuf,
          result};

use actix_web::{http::StatusCode,
                HttpResponse};

use crate::{bldr_core,
            builder_graph,
            db,
            hab_core,
            protocol};

#[derive(Debug)]
pub enum Error {
    BuilderCore(bldr_core::Error),
    BuilderGraph(builder_graph::Error),
    BusyWorkerUpsert(postgres::error::Error),
    BusyWorkerDelete(postgres::error::Error),
    BusyWorkersGet(postgres::error::Error),
    CaughtPanic(String, String),
    Conflict,
    Db(db::error::Error),
    DbPoolTimeout(r2d2::Error),
    DbTransaction(postgres::error::Error),
    DbTransactionStart(postgres::error::Error),
    DbTransactionCommit(postgres::error::Error),
    DieselError(diesel::result::Error),
    FromUtf8(std::string::FromUtf8Error),
    HabitatCore(hab_core::Error),
    InvalidUrl,
    IO(io::Error),
    JobGroupAudit(postgres::error::Error),
    JobGroupCreate(postgres::error::Error),
    JobGroupCancel(postgres::error::Error),
    JobGroupGet(postgres::error::Error),
    JobGroupOriginGet(postgres::error::Error),
    JobGroupPending(postgres::error::Error),
    JobGroupSetState(postgres::error::Error),
    JobGraphPackageInsert(postgres::error::Error),
    JobGraphPackageStats(postgres::error::Error),
    JobGraphPackagesGet(postgres::error::Error),
    JobGroupProjectSetState(postgres::error::Error),
    JobCreate(postgres::error::Error),
    JobGet(postgres::error::Error),
    JobLogArchive(u64, rusoto_core::RusotoError<rusoto_s3::PutObjectError>),
    JobLogRetrieval(u64, rusoto_core::RusotoError<rusoto_s3::GetObjectError>),
    JobMarkArchived(postgres::error::Error),
    JobPending(postgres::error::Error),
    JobReset(postgres::error::Error),
    JobSetLogUrl(postgres::error::Error),
    JobSetState(postgres::error::Error),
    SchedulerDbError(diesel::result::Error),
    SyncJobs(postgres::error::Error),
    LogDirDoesNotExist(PathBuf, io::Error),
    LogDirIsNotDir(PathBuf),
    LogDirNotWritable(PathBuf),
    NotFound,
    ParseError(chrono::format::ParseError),
    ParseVCSInstallationId(num::ParseIntError),
    Protobuf(protobuf::ProtobufError),
    Protocol(protocol::ProtocolError),
    System,
    UnknownVCS,
    UnknownJobGroup,
    UnknownJobGroupState,
    UnknownJobGraphPackage,
    UnknownJobGroupProjectState,
    UnknownJobState(protocol::ProtocolError),
    UnsupportedFeature(String),
    Utf8(std::str::Utf8Error),
    WorkerMgrDbError(diesel::result::Error),
    Zmq(zmq::Error),
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            Error::BuilderCore(ref e) => format!("{}", e),
            Error::BuilderGraph(ref e) => format!("{}", e),

            Error::BusyWorkerUpsert(ref e) => {
                format!("Database error creating or updating a busy worker, {}", e)
            }
            Error::BusyWorkerDelete(ref e) => {
                format!("Database error deleting a busy worker, {}", e)
            }
            Error::BusyWorkersGet(ref e) => {
                format!("Database error retrieving busy workers, {}", e)
            }
            Error::Conflict => "Entity conflict".to_string(),
            Error::CaughtPanic(ref msg, ref source) => {
                format!("Caught a panic: {}. {}", msg, source)
            }
            Error::Db(ref e) => format!("{}", e),
            Error::DbPoolTimeout(ref e) => {
                format!("Timeout getting connection from the database pool, {}", e)
            }
            Error::DbTransaction(ref e) => format!("Database transaction error, {}", e),
            Error::DbTransactionStart(ref e) => {
                format!("Failed to start database transaction, {}", e)
            }
            Error::DbTransactionCommit(ref e) => {
                format!("Failed to commit database transaction, {}", e)
            }
            Error::DieselError(ref e) => format!("{}", e),
            Error::FromUtf8(ref e) => format!("{}", e),
            Error::HabitatCore(ref e) => format!("{}", e),
            Error::InvalidUrl => "Bad URL!".to_string(),
            Error::IO(ref e) => format!("{}", e),
            Error::JobGroupAudit(ref e) => format!("Database error creating audit entry, {}", e),
            Error::JobGroupCreate(ref e) => format!("Database error creating a new group, {}", e),
            Error::JobGroupCancel(ref e) => format!("Database error canceling a job group, {}", e),
            Error::JobGroupGet(ref e) => format!("Database error getting group data, {}", e),
            Error::JobGroupOriginGet(ref e) => {
                format!("Database error getting group data for an origin, {}", e)
            }
            Error::JobGroupPending(ref e) => format!("Database error getting pending group, {}", e),
            Error::JobGroupSetState(ref e) => format!("Database error setting group state, {}", e),
            Error::JobGraphPackageInsert(ref e) => {
                format!("Database error inserting a new package, {}", e)
            }
            Error::JobGraphPackageStats(ref e) => {
                format!("Database error retrieving package statistics, {}", e)
            }
            Error::JobGraphPackagesGet(ref e) => {
                format!("Database error retrieving packages, {}", e)
            }
            Error::JobGroupProjectSetState(ref e) => {
                format!("Database error setting project state, {}", e)
            }
            Error::JobCreate(ref e) => format!("Database error creating a new job, {}", e),
            Error::JobGet(ref e) => format!("Database error getting job data, {}", e),
            Error::JobLogArchive(job_id, ref e) => {
                format!("Log archiving error for job {}, {}", job_id, e)
            }
            Error::JobLogRetrieval(job_id, ref e) => {
                format!("Log retrieval error for job {}, {}", job_id, e)
            }
            Error::JobMarkArchived(ref e) => {
                format!("Database error marking job as archived, {}", e)
            }
            Error::JobPending(ref e) => format!("Database error getting pending jobs, {}", e),
            Error::JobReset(ref e) => format!("Database error reseting jobs, {}", e),
            Error::JobSetLogUrl(ref e) => format!("Database error setting job log URL, {}", e),
            Error::JobSetState(ref e) => format!("Database error setting job state, {}", e),
            Error::SchedulerDbError(ref e) => format!("Database error setting in scheduler, {}", e),
            Error::SyncJobs(ref e) => format!("Database error retrieving sync jobs, {}", e),
            Error::LogDirDoesNotExist(ref path, ref e) => {
                format!("Build log directory {:?} doesn't exist!: {:?}", path, e)
            }
            Error::LogDirIsNotDir(ref path) => {
                format!("Build log directory {:?} is not a directory!", path)
            }
            Error::LogDirNotWritable(ref path) => {
                format!("Build log directory {:?} is not writable!", path)
            }
            Error::NotFound => "Entity not found".to_string(),
            Error::ParseError(ref e) => format!("Datetime could not be parsed, {}", e),
            Error::ParseVCSInstallationId(ref e) => {
                format!("VCS installation id could not be parsed as u64, {}", e)
            }
            Error::Protobuf(ref e) => format!("{}", e),
            Error::Protocol(ref e) => format!("{}", e),
            Error::System => "Internal error".to_string(),
            Error::UnknownJobGroup => "Unknown Group".to_string(),
            Error::UnknownJobGroupState => "Unknown Group State".to_string(),
            Error::UnknownJobGraphPackage => "Unknown Package".to_string(),
            Error::UnknownJobGroupProjectState => "Unknown Project State".to_string(),
            Error::UnknownVCS => "Unknown VCS".to_string(),
            Error::UnknownJobState(ref e) => format!("{}", e),
            Error::Utf8(ref e) => format!("{}", e),
            Error::WorkerMgrDbError(ref e) => format!("{}", e),
            Error::Zmq(ref e) => format!("{}", e),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for Error {}

impl Into<HttpResponse> for Error {
    fn into(self) -> HttpResponse {
        match self {
            Error::BuilderCore(ref e) => HttpResponse::new(bldr_core_err_to_http(e)),
            Error::Conflict => HttpResponse::new(StatusCode::CONFLICT),
            Error::DieselError(ref e) => HttpResponse::new(diesel_err_to_http(e)),
            Error::NotFound => HttpResponse::new(StatusCode::NOT_FOUND),
            Error::System => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),

            // Default
            _ => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

fn bldr_core_err_to_http(err: &bldr_core::Error) -> StatusCode {
    match err {
        bldr_core::error::Error::RpcError(code, _) => StatusCode::from_u16(*code).unwrap(),
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

impl From<bldr_core::Error> for Error {
    fn from(err: bldr_core::Error) -> Error { Error::BuilderCore(err) }
}

// Note: might be worth flattening out builder db errors into our db error types.
impl From<builder_graph::Error> for Error {
    fn from(err: builder_graph::Error) -> Error { Error::BuilderGraph(err) }
}

impl From<chrono::format::ParseError> for Error {
    fn from(err: chrono::format::ParseError) -> Error { Error::ParseError(err) }
}

impl From<hab_core::Error> for Error {
    fn from(err: hab_core::Error) -> Error { Error::HabitatCore(err) }
}

impl From<db::error::Error> for Error {
    fn from(err: db::error::Error) -> Self { Error::Db(err) }
}

impl From<diesel::result::Error> for Error {
    fn from(err: diesel::result::Error) -> Error { Error::DieselError(err) }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error { Error::IO(err) }
}

impl From<protobuf::ProtobufError> for Error {
    fn from(err: protobuf::ProtobufError) -> Error { Error::Protobuf(err) }
}

impl From<protocol::ProtocolError> for Error {
    fn from(err: protocol::ProtocolError) -> Self { Error::Protocol(err) }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Error { Error::FromUtf8(err) }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Error { Error::Utf8(err) }
}

impl From<zmq::Error> for Error {
    fn from(err: zmq::Error) -> Error { Error::Zmq(err) }
}
