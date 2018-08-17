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

use actix_web::http::{self, StatusCode};
use actix_web::FromRequest;
use actix_web::{HttpRequest, HttpResponse, Json, Path};
use protocol::originsrv::*;

use hab_core::package::ident;

use server::error::{Error, Result};
use server::framework::headers;
use server::framework::middleware::route_message;
use server::helpers;
use server::AppState;

/*

fn list_channels(req: &mut Request) -> IronResult<Response> {
    let origin_name = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let mut request = OriginChannelListRequest::new();
    request.set_include_sandbox_channels(false);

    match helpers::get_origin(req, &origin_name) {
        Ok(origin) => request.set_origin_id(origin.get_id()),
        Err(err) => return Ok(render_net_error(&err)),
    }

    // Pass ?sandbox=true to this endpoint to include sanbox channels in the list. They are not
    // there by default.
    if let Some(sandbox) = helpers::extract_query_value("sandbox", req) {
        if sandbox == "true" {
            request.set_include_sandbox_channels(true);
        }
    }

    match route_message::<OriginChannelListRequest, OriginChannelListResponse>(req, &request) {
        Ok(list) => {
            let list: Vec<OriginChannelIdent> = list
                .get_channels()
                .iter()
                .map(|channel| {
                    let mut ident = OriginChannelIdent::new();
                    ident.set_name(channel.get_name().to_string());
                    ident
                })
                .collect();
            let body = serde_json::to_string(&list).unwrap();
            let mut response = Response::with((status::Ok, body));
            dont_cache_response(&mut response);
            Ok(response)
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

fn create_channel(req: &mut Request) -> IronResult<Response> {
    let origin = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };
    let channel = match get_param(req, "channel") {
        Some(channel) => channel,
        None => return Ok(Response::with(status::BadRequest)),
    };

    match helpers::create_channel(req, &origin, &channel) {
        Ok(origin_channel) => Ok(render_json(status::Created, &origin_channel)),
        Err(err) => Ok(render_net_error(&err)),
    }
}

fn delete_channel(req: &mut Request) -> IronResult<Response> {
    let origin = match get_param(req, "origin") {
        Some(origin) => origin,
        None => return Ok(Response::with(status::BadRequest)),
    };
    let channel = match get_param(req, "channel") {
        Some(channel) => channel,
        None => return Ok(Response::with(status::BadRequest)),
    };

    let mut channel_req = OriginChannelGet::new();
    channel_req.set_origin_name(origin.clone());
    channel_req.set_name(channel.clone());
    match route_message::<OriginChannelGet, OriginChannel>(req, &channel_req) {
        Ok(origin_channel) => {
            // make sure the person trying to create the channel has access to do so
            if !check_origin_access(req, &origin).unwrap_or(false) {
                return Ok(Response::with(status::Forbidden));
            }

            // stable and unstable can't be deleted
            if channel == "stable" || channel == "unstable" {
                return Ok(Response::with(status::Forbidden));
            }

            let mut delete = OriginChannelDelete::new();
            delete.set_id(origin_channel.get_id());
            delete.set_origin_id(origin_channel.get_origin_id());
            match route_message::<OriginChannelDelete, NetOk>(req, &delete) {
                Ok(_) => Ok(Response::with(status::Ok)),
                Err(err) => return Ok(render_net_error(&err)),
            }
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}

fn promote_package(req: &mut Request) -> IronResult<Response> {
    let mut ident = OriginPackageIdent::new();
    match get_param(req, "origin") {
        Some(origin) => ident.set_origin(origin),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "pkg") {
        Some(pkg) => ident.set_name(pkg),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "version") {
        Some(version) => ident.set_version(version),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "release") {
        Some(release) => ident.set_release(release),
        None => return Ok(Response::with(status::BadRequest)),
    }
    let channel = match get_param(req, "channel") {
        Some(channel) => channel,
        None => return Ok(Response::with(status::BadRequest)),
    };

    if !check_origin_access(req, ident.get_origin()).unwrap_or(false) {
        let err = NetError::new(ErrCode::ACCESS_DENIED, "core:promote-package-to-channel:0");
        return Ok(render_net_error(&err));
    }

    let mut origin_get = OriginGet::new();
    origin_get.set_name(ident.get_origin().to_string());

    let origin_id = match route_message::<OriginGet, Origin>(req, &origin_get) {
        Ok(o) => o.get_id(),
        Err(err) => return Ok(render_net_error(&err)),
    };

    let mut channel_req = OriginChannelGet::new();
    channel_req.set_origin_name(ident.get_origin().to_string());
    channel_req.set_name(channel.to_string());

    let origin_channel = match route_message::<OriginChannelGet, OriginChannel>(req, &channel_req) {
        Ok(c) => c,
        Err(e) => {
            error!("Error retrieving channel: {:?}", &e);
            return Ok(render_net_error(&e));
        }
    };

    let mut request = OriginPackageGet::new();
    request.set_ident(ident.clone());
    request.set_visibilities(all_visibilities());

    let package = match route_message::<OriginPackageGet, OriginPackage>(req, &request) {
        Ok(p) => p,
        Err(e) => {
            error!("Error retrieving package: {:?}", &e);
            return Ok(render_net_error(&e));
        }
    };

    let mut promote = OriginPackagePromote::new();
    promote.set_channel_id(origin_channel.get_id());
    promote.set_package_id(package.get_id());
    promote.set_ident(ident.clone());

    match route_message::<OriginPackagePromote, NetOk>(req, &promote) {
        Ok(_) => match route_message::<OriginPackagePromote, NetOk>(req, &promote) {
            Ok(_) => match audit_package_rank_change(
                req,
                package.get_id(),
                origin_channel.get_id(),
                PackageChannelOperation::Promote,
                origin_id,
            ) {
                Ok(_) => Ok(Response::with(status::Ok)),
                Err(err) => return Ok(render_net_error(&err)),
            },
            Err(err) => return Ok(render_net_error(&err)),
        },
        Err(err) => Ok(render_net_error(&err)),
    }
}

fn demote_package(req: &mut Request) -> IronResult<Response> {
    let mut ident = OriginPackageIdent::new();
    match get_param(req, "origin") {
        Some(origin) => ident.set_origin(origin),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "pkg") {
        Some(pkg) => ident.set_name(pkg),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "version") {
        Some(version) => ident.set_version(version),
        None => return Ok(Response::with(status::BadRequest)),
    }
    match get_param(req, "release") {
        Some(release) => ident.set_release(release),
        None => return Ok(Response::with(status::BadRequest)),
    }
    let channel = match get_param(req, "channel") {
        Some(channel) => channel,
        None => return Ok(Response::with(status::BadRequest)),
    };

    // you can't demote from "unstable"
    if channel == "unstable" {
        return Ok(Response::with(status::Forbidden));
    }

    if !check_origin_access(req, &ident.get_origin()).unwrap_or(false) {
        return Ok(Response::with(status::Forbidden));
    }

    let mut origin_get = OriginGet::new();
    origin_get.set_name(ident.get_origin().to_string());

    let origin_id = match route_message::<OriginGet, Origin>(req, &origin_get) {
        Ok(o) => o.get_id(),
        Err(err) => return Ok(render_net_error(&err)),
    };

    let mut channel_req = OriginChannelGet::new();
    channel_req.set_origin_name(ident.get_origin().to_string());
    channel_req.set_name(channel);
    match route_message::<OriginChannelGet, OriginChannel>(req, &channel_req) {
        Ok(origin_channel) => {
            let mut request = OriginPackageGet::new();
            request.set_ident(ident.clone());
            request.set_visibilities(all_visibilities());
            match route_message::<OriginPackageGet, OriginPackage>(req, &request) {
                Ok(package) => {
                    let mut demote = OriginPackageDemote::new();
                    demote.set_channel_id(origin_channel.get_id());
                    demote.set_package_id(package.get_id());
                    demote.set_ident(ident.clone());
                    match route_message::<OriginPackageDemote, NetOk>(req, &demote) {
                        Ok(_) => match audit_package_rank_change(
                            req,
                            package.get_id(),
                            origin_channel.get_id(),
                            PackageChannelOperation::Demote,
                            origin_id,
                        ) {
                            Ok(_) => Ok(Response::with(status::Ok)),
                            Err(err) => return Ok(render_net_error(&err)),
                        },
                        Err(err) => return Ok(render_net_error(&err)),
                    }
                }
                Err(err) => Ok(render_net_error(&err)),
            }
        }
        Err(err) => Ok(render_net_error(&err)),
    }
}


fn audit_package_rank_change(
    req: &mut Request,
    package_id: u64,
    channel_id: u64,
    operation: PackageChannelOperation,
    origin_id: u64,
) -> NetResult<NetOk> {
    let mut audit = PackageChannelAudit::new();
    audit.set_package_id(package_id);
    audit.set_channel_id(channel_id);
    audit.set_operation(operation);

    let jgt = trigger_from_request(req);
    audit.set_trigger(PackageChannelTrigger::from(jgt));

    let (session_id, session_name) = get_session_id_and_name(req);

    audit.set_requester_id(session_id);
    audit.set_requester_name(session_name);
    audit.set_origin_id(origin_id);

    route_message::<PackageChannelAudit, NetOk>(req, &audit)
}

*/
