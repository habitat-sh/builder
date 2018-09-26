// Copyright (c) 2018 Chef Software Inc. and/or applicable contributors
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

use actix_web::http::{self, Method, StatusCode};
use actix_web::{error, App, HttpRequest, HttpResponse, Path, Query};
use actix_web::{AsyncResponder, FromRequest, FutureResponse};
use futures::{future, Future};

use server::authorize::authorize_session;
use server::framework::headers;
use server::models::channel::{CreateChannel, DeleteChannel, ListChannels};
use server::AppState;

// Query param containers
#[derive(Debug, Default, Clone, Deserialize)]
struct SandboxBool {
    #[serde(default)]
    is_set: bool,
}

pub struct Channels;

impl Channels {
    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.route("/depot/channels/{origin}", Method::GET, get_channels)
            .route(
                "/depot/channels/{origin}/{channel}",
                Method::POST,
                create_channel,
            ).route(
                "/depot/channels/{origin}/{channel}",
                Method::DELETE,
                delete_channel,
            )
    }
}

//
// Route handlers - these functions can return any Responder trait
//
fn get_channels(
    (req, sandbox): (HttpRequest<AppState>, Query<SandboxBool>),
) -> FutureResponse<HttpResponse> {
    let origin = Path::<(String)>::extract(&req).unwrap().into_inner();

    req.state()
        .db
        .send(ListChannels {
            origin: origin,
            include_sandbox_channels: sandbox.is_set,
        }).from_err()
        .and_then(|res| match res {
            Ok(list) => {
                // TED: This is to maintain backwards API compat while killing some proto definitions
                // currently the output looks like [{"name": "foo"}] when it probably should be ["foo"]
                #[derive(Serialize)]
                struct Temp {
                    name: String,
                }
                let ident_list: Vec<Temp> = list
                    .iter()
                    .map(|channel| Temp {
                        name: channel.name.clone(),
                    }).collect();
                Ok(HttpResponse::Ok()
                    .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                    .json(ident_list))
            }
            Err(_err) => Ok(HttpResponse::InternalServerError().into()),
        }).responder()
}

fn create_channel(req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    let (origin, channel) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner();

    let session_id = match authorize_session(&req, Some(&origin)) {
        Ok(session_id) => session_id as i64,
        Err(e) => return future::err(error::ErrorUnauthorized(e)).responder(),
    };

    req.state()
        .db
        .send(CreateChannel {
            name: channel,
            origin: origin,
            owner_id: session_id,
        }).from_err()
        .and_then(|res| match res {
            Ok(channel) => Ok(HttpResponse::Created().json(channel)),
            Err(e) => Err(e),
        }).responder()
}

fn delete_channel(req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    let (origin, channel) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return future::err(error::ErrorUnauthorized(err)).responder();
    }

    if channel == "stable" || channel == "unstable" {
        return future::err(error::ErrorForbidden(format!("{} is protected", channel))).responder();
    }

    req.state()
        .db
        .send(DeleteChannel {
            origin: origin,
            channel: channel,
        }).from_err()
        .and_then(|res| match res {
            Ok(_) => Ok(HttpResponse::new(StatusCode::OK)),
            Err(e) => Err(e),
        }).responder()
}
