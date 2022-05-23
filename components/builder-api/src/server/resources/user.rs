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

use actix_web::{web::{self,
                      Data,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};

use crate::db::models::{invitations::OriginInvitation,
                        origin::Origin};

use crate::server::{authorize::authorize_session,
                    error::Error,
                    AppState};

pub struct User {}

impl User {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/user/invitations", web::get().to(get_invitations))
           .route("/user/origins", web::get().to(get_origins));
    }
}

// Route handlers - these functions can return any Responder trait
//
#[allow(clippy::needless_pass_by_value)]
async fn get_invitations(req: HttpRequest, state: Data<AppState>) -> HttpResponse {
    let account_id = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginInvitation::list_by_account(account_id, &*conn) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_origins(req: HttpRequest, state: Data<AppState>) -> HttpResponse {
    let account_id = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id() as i64,
        Err(err) => return err.into(),
    };

    let conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Origin::list(account_id, &*conn) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}
