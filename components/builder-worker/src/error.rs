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
          path::PathBuf,
          result};

use crate::{bldr_core,
            hab_core,
            protocol};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Error {
    BuildEnvFile(PathBuf, io::Error),
    BuildFailure(i32),
    BuilderCore(bldr_core::Error),
    CannotAddCreds,
    Chown(PathBuf, u32, u32, io::Error),
    ChownWait(io::Error),
    CreateDirectory(PathBuf, io::Error),
    Exporter(io::Error),
    ExportFailure(i32),
    Git(git2::Error),
    GithubAppAuthErr(github_api_client::HubError),
    HabitatCore(hab_core::Error),
    InvalidIntegrations(String),
    NotHTTPSCloneUrl(url::Url),
    Protobuf(protobuf::ProtobufError),
    Protocol(protocol::ProtocolError),
    StreamLine(io::Error),
    StreamTargetSend(zmq::Error),
    StudioBuild(PathBuf, io::Error),
    StudioTeardown(PathBuf, io::Error),
    UrlParseError(url::ParseError),
    WorkspaceSetup(String, io::Error),
    WorkspaceTeardown(String, io::Error),
    Zmq(zmq::Error),
    MpscAsync(futures_channel::mpsc::SendError),
    JobCanceled,
}

#[allow(clippy::many_single_char_names)]
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            Error::BuildEnvFile(ref p, ref e) => {
                format!("Unable to read workspace build env file, {}, {}",
                        p.display(),
                        e)
            }
            Error::BuildFailure(ref e) => {
                format!("Build studio exited with non-zero exit code, {}", e)
            }
            Error::BuilderCore(ref e) => format!("{}", e),
            Error::CannotAddCreds => "Cannot add credentials to url".to_string(),
            Error::Chown(ref p, ref u, ref g, ref e) => {
                format!("Unable to recursively chown path, {} with '{}:{}', {}",
                        p.display(),
                        u,
                        g,
                        e)
            }
            Error::ChownWait(ref e) => format!("Unable to complete chown process, {}", e),
            Error::CreateDirectory(ref p, ref e) => {
                format!("Unable to create directory {}, err={}", p.display(), e)
            }
            Error::Exporter(ref e) => {
                format!("Unable to spawn or pipe data from exporter proc, {}", e)
            }
            Error::ExportFailure(ref e) => {
                format!("Docker export exited with non-zero exit code, {}", e)
            }
            Error::Git(ref e) => format!("{}", e),
            Error::GithubAppAuthErr(ref e) => format!("{}", e),
            Error::HabitatCore(ref e) => format!("{}", e),
            Error::InvalidIntegrations(ref s) => format!("Invalid integration: {}", s),
            Error::NotHTTPSCloneUrl(ref e) => {
                format!("Attempted to clone {}. Only HTTPS clone urls are supported",
                        e)
            }
            Error::Protobuf(ref e) => format!("{}", e),
            Error::Protocol(ref e) => format!("{}", e),
            Error::StreamLine(ref e) => {
                format!("Error while reading a line while consuming an output stream, err={}",
                        e)
            }
            Error::StreamTargetSend(ref e) => {
                format!("Error while writing a message to the job stream, err={}", e)
            }
            Error::StudioBuild(ref p, ref e) => {
                format!("Error while running studio build at {}, err={}",
                        p.display(),
                        e)
            }
            Error::StudioTeardown(ref p, ref e) => {
                format!("Error while tearing down studio at {}, err={}",
                        p.display(),
                        e)
            }
            Error::UrlParseError(ref e) => format!("{}", e),
            Error::WorkspaceSetup(ref p, ref e) => {
                format!("Error while setting up workspace at {}, err={}", p, e)
            }
            Error::WorkspaceTeardown(ref p, ref e) => {
                format!("Error while tearing down workspace at {}, err={}", p, e)
            }
            Error::Zmq(ref e) => format!("{}", e),
            Error::MpscAsync(ref e) => format!("{}", e),
            Error::JobCanceled => "Job was canceled".to_string(),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for Error {}

impl From<bldr_core::Error> for Error {
    fn from(err: bldr_core::Error) -> Error { Error::BuilderCore(err) }
}

impl From<hab_core::Error> for Error {
    fn from(err: hab_core::Error) -> Error { Error::HabitatCore(err) }
}

impl From<github_api_client::HubError> for Error {
    fn from(err: github_api_client::HubError) -> Error { Error::GithubAppAuthErr(err) }
}

impl From<protobuf::ProtobufError> for Error {
    fn from(err: protobuf::ProtobufError) -> Error { Error::Protobuf(err) }
}

impl From<protocol::ProtocolError> for Error {
    fn from(err: protocol::ProtocolError) -> Self { Error::Protocol(err) }
}

impl From<zmq::Error> for Error {
    fn from(err: zmq::Error) -> Error { Error::Zmq(err) }
}
