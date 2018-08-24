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

mod handlers;

use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::sync::RwLock;
use std::time::{Duration, Instant};

use base64;
use hab_net::app::prelude::*;
use hab_net::privilege::FeatureFlags;
use protobuf::Message;
use protocol::{message, sessionsrv as proto};

use config::Config;
use data_store::DataStore;
use error::{SrvError, SrvResult};

lazy_static! {
    static ref DISPATCH_TABLE: DispatchTable<SessionSrv> = {
        let mut map = DispatchTable::new();
        map.register(
            proto::AccountGet::descriptor_static(),
            handlers::account_get,
        );
        map.register(
            proto::AccountGetId::descriptor_static(),
            handlers::account_get_id,
        );
        map.register(
            proto::AccountCreate::descriptor_static(),
            handlers::account_create,
        );
        map.register(
            proto::AccountUpdate::descriptor_static(),
            handlers::account_update,
        );
        map.register(
            proto::AccountFindOrCreate::descriptor_static(),
            handlers::account_find_or_create,
        );
        map.register(
            proto::AccountTokenCreate::descriptor_static(),
            handlers::account_token_create,
        );
        map.register(
            proto::AccountTokenRevoke::descriptor_static(),
            handlers::account_token_revoke,
        );
        map.register(
            proto::AccountTokensGet::descriptor_static(),
            handlers::account_tokens_get,
        );
        map.register(
            proto::AccountTokenValidate::descriptor_static(),
            handlers::account_token_validate,
        );
        map.register(
            proto::SessionCreate::descriptor_static(),
            handlers::session_create,
        );
        map.register(
            proto::SessionGet::descriptor_static(),
            handlers::session_get,
        );
        map
    };
    static ref SESSION_DURATION: Duration = { Duration::from_secs(1 * 24 * 60 * 60) };
}

#[derive(Clone, Debug)]
pub struct Session {
    pub created_at: Instant,
    encoded_token: String,
    inner: proto::Session,
}

impl Session {
    pub fn build(
        mut msg: proto::SessionCreate,
        mut account: proto::Account,
        flags: FeatureFlags,
    ) -> SrvResult<Self> {
        let mut session = proto::Session::new();
        let mut token = proto::SessionToken::new();
        token.set_account_id(account.get_id());
        token.set_extern_id(msg.get_extern_id().to_string());
        token.set_provider(msg.get_provider());
        token.set_token(msg.get_token().to_string().into_bytes());

        let encoded_token = encode_token(&token)?;
        session.set_id(account.get_id());
        session.set_email(account.take_email());
        session.set_name(account.take_name());
        session.set_token(encoded_token.clone());
        session.set_flags(flags.bits());
        session.set_oauth_token(msg.take_token());
        Ok(Session {
            created_at: Instant::now(),
            encoded_token: encoded_token,
            inner: session,
        })
    }

    pub fn expired(&self) -> bool {
        self.created_at.elapsed() >= *SESSION_DURATION
    }
}

impl Borrow<str> for Session {
    fn borrow(&self) -> &str {
        &self.encoded_token
    }
}

impl Deref for Session {
    type Target = proto::Session;

    fn deref(&self) -> &proto::Session {
        &self.inner
    }
}

impl DerefMut for Session {
    fn deref_mut(&mut self) -> &mut proto::Session {
        &mut self.inner
    }
}

impl Eq for Session {}

impl Hash for Session {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.encoded_token.hash(state);
    }
}

impl PartialEq for Session {
    fn eq(&self, other: &Session) -> bool {
        self.encoded_token == other.encoded_token
    }
}

#[derive(Clone)]
pub struct ServerState {
    datastore: DataStore,
    sessions: Arc<Box<RwLock<HashSet<Session>>>>,
    tokens: Arc<Box<RwLock<HashMap<u64, Option<String>>>>>,
}

impl ServerState {
    fn new(cfg: Config) -> SrvResult<Self> {
        let datastore = DataStore::new(&cfg.datastore)?;

        Ok(ServerState {
            datastore: datastore,
            sessions: Arc::new(Box::new(RwLock::new(HashSet::default()))),
            tokens: Arc::new(Box::new(RwLock::new(HashMap::new()))), // TBD: Handle multiple tokens / account
        })
    }
}

impl AppState for ServerState {
    type Error = SrvError;
    type InitState = Self;

    fn build(init_state: Self::InitState) -> SrvResult<Self> {
        Ok(init_state)
    }
}

struct SessionSrv;
impl Dispatcher for SessionSrv {
    const APP_NAME: &'static str = "builder-sessionsrv";
    const PROTOCOL: Protocol = Protocol::SessionSrv;

    type Config = Config;
    type Error = SrvError;
    type State = ServerState;

    fn app_init(
        cfg: Self::Config,
        _: Arc<String>,
    ) -> SrvResult<<Self::State as AppState>::InitState> {
        let state = ServerState::new(cfg)?;
        Ok(state)
    }

    fn dispatch_table() -> &'static DispatchTable<Self> {
        &DISPATCH_TABLE
    }
}

pub fn encode_token(token: &proto::SessionToken) -> SrvResult<String> {
    let bytes = message::encode(token)?;
    Ok(base64::encode(&bytes))
}

pub fn decode_token(value: &str) -> SrvResult<proto::SessionToken> {
    let decoded = base64::decode(value).unwrap();
    let token = message::decode(&decoded)?;
    Ok(token)
}

pub fn run(config: Config) -> AppResult<(), SrvError> {
    app_start::<SessionSrv>(config)
}

pub fn migrate(config: Config) -> SrvResult<()> {
    let ds = DataStore::new(&config.datastore)?;
    ds.setup()
}

#[cfg(test)]
mod test {
    use super::*;
    use protocol::sessionsrv as proto;

    #[test]
    fn decode_session_token() {
        let t = "CL3Ag7z4tvaAChCUpgMYACIoZDFmODI3NDc3YTk4ODUyM2E0ZGUyY2JmZjgwNWEyN2ZmOTZkNmIzNQ==";
        let token = decode_token(t).unwrap();
        assert_eq!(token.get_account_id(), 721096797631602749);
        assert_eq!(token.get_extern_id(), "54036".to_string());
        assert_eq!(token.get_provider(), proto::OAuthProvider::GitHub);
        assert_eq!(
            String::from_utf8_lossy(token.get_token()),
            "d1f827477a988523a4de2cbff805a27ff96d6b35"
        );
    }

    #[test]
    fn encode_session_token() {
        let mut token = proto::SessionToken::new();
        token.set_account_id(721096797631602749);
        token.set_extern_id("54036".to_string());
        token.set_provider(proto::OAuthProvider::GitHub);
        token.set_token(
            "d1f827477a988523a4de2cbff805a27ff96d6b35"
                .to_string()
                .into_bytes(),
        );
        let encoded = encode_token(&token).unwrap();
        assert_eq!(
            encoded,
            "CL3Ag7z4tvaAChCUpgMYACIoZDFmODI3NDc3YTk4ODUyM2E0ZGUyY2JmZjgwNWEyN2ZmOTZkNmIzNQ=="
        );
    }
}
