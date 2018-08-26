// Copyright (c) 2016 Chef Software Inc. and/or applicable contributors
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

//! A module containing the HTTP server and handlers for servicing client requests

use hab_net::{ErrCode, NetError};

use iron::headers::ContentType;
use iron::mime::{Attr, Mime, SubLevel, TopLevel, Value};
use iron::modifiers::Header;
use iron::prelude::*;
use iron::status;
use iron::status::Status;
use serde::Serialize;
use serde_json;

pub fn net_err_to_http(err: ErrCode) -> Status {
    match err {
        ErrCode::TIMEOUT => Status::GatewayTimeout,
        ErrCode::REMOTE_REJECTED => Status::NotAcceptable,
        ErrCode::ENTITY_NOT_FOUND => Status::NotFound,
        ErrCode::ENTITY_CONFLICT => Status::Conflict,

        ErrCode::ACCESS_DENIED | ErrCode::SESSION_EXPIRED => Status::Unauthorized,

        ErrCode::BAD_REMOTE_REPLY | ErrCode::SECRET_KEY_FETCH | ErrCode::VCS_CLONE => {
            Status::BadGateway
        }

        ErrCode::NO_SHARD | ErrCode::SOCK | ErrCode::REMOTE_UNAVAILABLE => {
            Status::ServiceUnavailable
        }

        ErrCode::BAD_TOKEN => Status::Forbidden,
        ErrCode::GROUP_NOT_COMPLETE => Status::UnprocessableEntity,
        ErrCode::PARTIAL_JOB_GROUP_PROMOTE => Status::PartialContent,

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
        | ErrCode::REG_NOT_FOUND => Status::InternalServerError,
    }
}

pub fn render_json<T>(status: status::Status, response: &T) -> Response
where
    T: Serialize,
{
    let encoded = serde_json::to_string(response).unwrap();
    let headers = Header(ContentType(Mime(
        TopLevel::Application,
        SubLevel::Json,
        vec![(Attr::Charset, Value::Utf8)],
    )));
    Response::with((status, encoded, headers))
}

/// Return an IronResult containing the body of a NetError and the appropriate HTTP response status
/// for the corresponding NetError.
///
/// For example, a NetError::ENTITY_NOT_FOUND will result in an HTTP response containing the body
/// of the NetError with an HTTP status of 404.
///
/// # Panics
///
/// * The given encoded message was not a NetError
/// * The given message could not be decoded
/// * The NetError could not be encoded to JSON
pub fn render_net_error(err: &NetError) -> Response {
    render_json(net_err_to_http(err.code()), err)
}
