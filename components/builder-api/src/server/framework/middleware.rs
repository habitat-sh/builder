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

use actix_web::middleware::{Middleware, Response, Started};
use actix_web::{App, HttpRequest, HttpResponse, Result};

use protobuf;

use hab_net::conn::RouteClient;
use hab_net::{ErrCode, NetError, NetResult};
use oauth_client::types::OAuth2User;
use protocol::sessionsrv::{OAuthProvider, Session, SessionCreate, SessionType};
use protocol::Routable;

use server::services::route_broker::RouteBroker;
use server::AppState;

// Router client
pub struct XRouteClient;

impl<S> Middleware<S> for XRouteClient {
    fn start(&self, req: &HttpRequest<S>) -> Result<Started> {
        let conn = RouteBroker::connect().unwrap();
        req.extensions_mut().insert::<RouteClient>(conn);
        Ok(Started::Done)
    }
}

pub fn route_message<M, R>(req: &HttpRequest<AppState>, msg: &M) -> NetResult<R>
where
    M: Routable,
    R: protobuf::Message,
{
    req.extensions_mut()
        .get_mut::<RouteClient>()
        .expect("no XRouteClient extension in request")
        .route::<M, R>(msg)
}

/* OLD 

pub fn route_message<M, R>(req: &mut Request, msg: &M) -> NetResult<R>
where
    M: Routable,
    R: protobuf::Message,
{
    req.extensions
        .get_mut::<XRouteClient>()
        .expect("no XRouteClient extension in request")
        .route::<M, R>(msg)
}

/// Wrapper around the standard `iron::Chain` to assist in adding middleware on a per-handler basis
pub struct XHandler(Chain);

impl XHandler {
    /// Create a new XHandler
    pub fn new<H>(handler: H) -> Self
    where
        H: Handler,
    {
        XHandler(Chain::new(handler))
    }

    /// Add one or more before-middleware to the handler's chain
    pub fn before<M>(mut self, middleware: M) -> Self
    where
        M: BeforeMiddleware,
    {
        self.0.link_before(middleware);
        self
    }

    /// Add one or more after-middleware to the handler's chain
    pub fn after<M>(mut self, middleware: M) -> Self
    where
        M: AfterMiddleware,
    {
        self.0.link_after(middleware);
        self
    }

    /// Ad one or more around-middleware to the handler's chain
    pub fn around<M>(mut self, middleware: M) -> Self
    where
        M: AroundMiddleware,
    {
        self.0.link_around(middleware);
        self
    }
}

impl Handler for XHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        self.0.handle(req)
    }
}

pub struct OAuthCli;

impl Key for OAuthCli {
    type Value = OAuth2Client;
}

pub struct GitHubCli;

impl Key for GitHubCli {
    type Value = GitHubClient;
}

pub struct SegmentCli;

impl Key for SegmentCli {
    type Value = SegmentClient;
}

pub struct XRouteClient;

impl Key for XRouteClient {
    type Value = RouteClient;
}

impl BeforeMiddleware for XRouteClient {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        let conn = RouteBroker::connect().unwrap();
        req.extensions.insert::<XRouteClient>(conn);
        Ok(())
    }
}

#[derive(Clone)]
pub struct Authenticated {
    features: FeatureFlags,
    key_dir: PathBuf,
    optional: bool,
}

impl Authenticated {
    pub fn new(key_dir: PathBuf) -> Authenticated {
        Authenticated {
            features: FeatureFlags::empty(),
            key_dir: key_dir,
            optional: false,
        }
    }

    pub fn require(mut self, flag: FeatureFlags) -> Self {
        self.features.insert(flag);
        self
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    fn authenticate(&self, req: &mut Request, token: &str) -> IronResult<Session> {
        // Test hook - always create a valid session
        if env::var_os("HAB_FUNC_TEST").is_some() {
            debug!(
                "HAB_FUNC_TEST: {:?}; calling session_create_short_circuit",
                env::var_os("HAB_FUNC_TEST")
            );
            return session_create_short_circuit(req, token);
        };

        // Check for a valid personal access token
        if bldr_core::access_token::is_access_token(token) {
            match bldr_core::access_token::validate_access_token(&self.key_dir, token) {
                Ok(session) => {
                    if !revocation_check(req, session.get_id(), token) {
                        let err = NetError::new(ErrCode::BAD_TOKEN, "net:auth:revoked-token");
                        return Err(IronError::new(err, Status::Forbidden));
                    } else {
                        return Ok(session);
                    }
                }
                Err(bldr_core::Error::TokenExpired) => {
                    let err = NetError::new(ErrCode::BAD_TOKEN, "net:auth:expired-token");
                    return Err(IronError::new(err, Status::Forbidden));
                }
                Err(e) => {
                    warn!("Unable to validate access token, err={:?}", e);
                    let err = NetError::new(ErrCode::BAD_TOKEN, "net:auth:bad-token");
                    return Err(IronError::new(err, Status::Forbidden));
                }
            }
        };

        // Check for internal sessionsrv token
        let decoded_token = match base64::decode(token) {
            Ok(decoded_token) => decoded_token,
            Err(e) => {
                warn!("Failed to base64 decode token, err={:?}", e);
                let err = NetError::new(ErrCode::BAD_TOKEN, "net:auth:decode:1");
                return Err(IronError::new(err, Status::Forbidden));
            }
        };

        match message::decode(&decoded_token) {
            Ok(session_token) => session_validate(req, self.features, session_token),
            Err(e) => {
                warn!("Failed to decode token, err={:?}", e);
                let err = NetError::new(ErrCode::BAD_TOKEN, "net:auth:decode:2");
                return Err(IronError::new(err, Status::Forbidden));
            }
        }
    }
}

impl Key for Authenticated {
    type Value = Session;
}

impl BeforeMiddleware for Authenticated {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        let token = match req.headers.get::<Authorization<Bearer>>() {
            Some(&Authorization(Bearer { ref token })) => token.to_owned(),
            _ => {
                if self.optional {
                    return Ok(());
                } else {
                    let err = NetError::new(ErrCode::ACCESS_DENIED, "net:auth:no-token");
                    return Err(IronError::new(err, Status::Unauthorized));
                }
            }
        };

        let session = self.authenticate(req, &token)?;
        req.extensions.insert::<Self>(session);
        Ok(())
    }
}

pub struct Cors;

impl AfterMiddleware for Cors {
    fn after(&self, _req: &mut Request, mut res: Response) -> IronResult<Response> {
        res.headers.set(headers::AccessControlAllowOrigin::Any);
        res.headers.set(headers::AccessControlAllowHeaders(vec![
            UniCase("authorization".to_string()),
            UniCase("range".to_string()),
        ]));
        res.headers.set(headers::AccessControlAllowMethods(vec![
            Method::Put,
            Method::Delete,
            Method::Patch,
        ]));
        res.headers
            .set(headers::AccessControlExposeHeaders(vec![UniCase(
                "content-disposition".to_string(),
            )]));
        Ok(res)
    }
}

pub fn revocation_check(req: &mut Request, account_id: u64, token: &str) -> bool {
    let mut request = AccountTokenValidate::new();
    request.set_account_id(account_id);
    request.set_token(token.to_owned());
    let conn = req.extensions.get_mut::<XRouteClient>().unwrap();
    match conn.route::<AccountTokenValidate, NetOk>(&request) {
        Ok(_) => true,
        Err(e) => {
            warn!("Unable to validate token (possibly revoked): {:?}", e);
            false
        }
    }
}

pub fn session_validate(
    req: &mut Request,
    features: FeatureFlags,
    token: SessionToken,
) -> IronResult<Session> {
    let mut request = SessionGet::new();
    request.set_token(token);
    let conn = req.extensions.get_mut::<XRouteClient>().unwrap();
    match conn.route::<SessionGet, Session>(&request) {
        Ok(session) => {
            let flags = FeatureFlags::from_bits(session.get_flags()).unwrap();
            if !flags.contains(features) {
                let err = NetError::new(ErrCode::ACCESS_DENIED, "net:auth:feature-flags");
                return Err(IronError::new(err, Status::Forbidden));
            }
            Ok(session)
        }
        Err(err) => {
            let status = net_err_to_http(err.get_code());
            let body = itry!(serde_json::to_string(&err));
            Err(IronError::new(err, (body, status)))
        }
    }
}
*/

pub fn session_create_oauth(
    req: &HttpRequest<AppState>,
    token: &str,
    user: &OAuth2User,
    provider: &str,
) -> NetResult<Session> {
    let mut request = SessionCreate::new();
    request.set_session_type(SessionType::User);
    request.set_token(token.to_owned());
    request.set_extern_id(user.id.clone());
    request.set_name(user.username.clone());

    match provider.parse::<OAuthProvider>() {
        Ok(p) => request.set_provider(p),
        Err(e) => {
            warn!(
                "Error parsing oauth provider: provider={}, err={:?}",
                provider, e
            );
            return Err(NetError::new(ErrCode::BUG, "session_create_oauth:1"));
        }
    }

    if let Some(ref email) = user.email {
        request.set_email(email.clone());
    }

    route_message::<SessionCreate, Session>(req, &request)
}

pub fn session_create_short_circuit(
    req: &HttpRequest<AppState>,
    token: &str,
) -> NetResult<Session> {
    let request = match token.as_ref() {
        "bobo" => {
            let mut request = SessionCreate::new();
            request.set_session_type(SessionType::User);
            request.set_extern_id("0".to_string());
            request.set_email("bobo@example.com".to_string());
            request.set_name("bobo".to_string());
            request.set_provider(OAuthProvider::GitHub);
            request
        }
        "mystique" => {
            let mut request = SessionCreate::new();
            request.set_session_type(SessionType::User);
            request.set_extern_id("1".to_string());
            request.set_email("mystique@example.com".to_string());
            request.set_name("mystique".to_string());
            request.set_provider(OAuthProvider::GitHub);
            request
        }
        "hank" => {
            let mut request = SessionCreate::new();
            request.set_extern_id("2".to_string());
            request.set_email("hank@example.com".to_string());
            request.set_name("hank".to_string());
            request.set_provider(OAuthProvider::GitHub);
            request
        }
        user => {
            error!("Unexpected short circuit token {:?}", user);
            return Err(NetError::new(
                ErrCode::BUG,
                "net:session-short-circuit:unknown-token",
            ));
        }
    };

    route_message::<SessionCreate, Session>(req, &request)
}
