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
// This needs to be public so it can be used by the op tool
pub mod session;
mod session_handlers;

use self::session::Session;
use hab_net::app::prelude::*;
use protobuf::Message;
use protocol::originsrv::*;
use std::collections::HashSet;
use std::sync::RwLock;

use config::Config;
use data_store::DataStore;
use error::{SrvError, SrvResult};

lazy_static! {
    static ref DISPATCH_TABLE: DispatchTable<OriginSrv> = {
        let mut map = DispatchTable::new();
        map.register(
            CheckOriginAccessRequest::descriptor_static(),
            handlers::origin_check_access,
        );
        map.register(
            CheckOriginOwnerRequest::descriptor_static(),
            handlers::origin_check_owner,
        );
        map.register(OriginCreate::descriptor_static(), handlers::origin_create);
        map.register(OriginUpdate::descriptor_static(), handlers::origin_update);
        map.register(OriginGet::descriptor_static(), handlers::origin_get);
        map.register(
            OriginIntegrationGetNames::descriptor_static(),
            handlers::origin_integration_get_names,
        );
        map.register(
            OriginIntegrationCreate::descriptor_static(),
            handlers::origin_integration_create,
        );
        map.register(
            OriginIntegrationDelete::descriptor_static(),
            handlers::origin_integration_delete,
        );
        map.register(
            OriginIntegrationGet::descriptor_static(),
            handlers::origin_integration_get,
        );
        map.register(
            OriginIntegrationRequest::descriptor_static(),
            handlers::origin_integration_request,
        );
        map.register(
            OriginInvitationAcceptRequest::descriptor_static(),
            handlers::origin_invitation_accept,
        );
        map.register(
            OriginInvitationCreate::descriptor_static(),
            handlers::origin_invitation_create,
        );
        map.register(
            OriginInvitationIgnoreRequest::descriptor_static(),
            handlers::origin_invitation_ignore,
        );
        map.register(
            OriginInvitationListRequest::descriptor_static(),
            handlers::origin_invitation_list,
        );
        map.register(
            AccountInvitationListRequest::descriptor_static(),
            handlers::account_invitation_list,
        );
        map.register(
            OriginInvitationRescindRequest::descriptor_static(),
            handlers::origin_invitation_rescind,
        );
        map.register(
            OriginMemberListRequest::descriptor_static(),
            handlers::origin_member_list,
        );
        map.register(
            OriginPackageUpdate::descriptor_static(),
            handlers::origin_package_update,
        );
        map.register(
            OriginPrivateEncryptionKeyCreate::descriptor_static(),
            handlers::origin_private_encryption_key_create,
        );
        map.register(
            OriginPrivateEncryptionKeyGet::descriptor_static(),
            handlers::origin_private_encryption_key_get,
        );
        map.register(
            OriginPublicEncryptionKeyCreate::descriptor_static(),
            handlers::origin_public_encryption_key_create,
        );
        map.register(
            OriginPublicEncryptionKeyGet::descriptor_static(),
            handlers::origin_public_encryption_key_get,
        );
        map.register(
            OriginPublicEncryptionKeyLatestGet::descriptor_static(),
            handlers::origin_public_encryption_key_latest_get,
        );
        map.register(
            OriginPublicEncryptionKeyListRequest::descriptor_static(),
            handlers::origin_public_encryption_key_list,
        );
        map.register(
            OriginPrivateSigningKeyCreate::descriptor_static(),
            handlers::origin_secret_key_create,
        );
        map.register(
            OriginPrivateSigningKeyGet::descriptor_static(),
            handlers::origin_secret_key_get,
        );
        map.register(
            OriginPublicSigningKeyCreate::descriptor_static(),
            handlers::origin_public_key_create,
        );
        map.register(
            OriginPublicSigningKeyGet::descriptor_static(),
            handlers::origin_public_key_get,
        );
        map.register(
            OriginPublicSigningKeyLatestGet::descriptor_static(),
            handlers::origin_public_key_latest_get,
        );
        map.register(
            OriginPublicSigningKeyListRequest::descriptor_static(),
            handlers::origin_public_key_list,
        );
        map.register(
            OriginProjectCreate::descriptor_static(),
            handlers::project_create,
        );
        map.register(
            OriginProjectDelete::descriptor_static(),
            handlers::project_delete,
        );
        map.register(OriginProjectGet::descriptor_static(), handlers::project_get);
        map.register(
            OriginProjectListGet::descriptor_static(),
            handlers::project_list_get,
        );
        map.register(
            OriginProjectUpdate::descriptor_static(),
            handlers::project_update,
        );
        map.register(
            OriginProjectIntegrationCreate::descriptor_static(),
            handlers::project_integration_create,
        );
        map.register(
            OriginProjectIntegrationDelete::descriptor_static(),
            handlers::project_integration_delete,
        );
        map.register(
            OriginProjectIntegrationGet::descriptor_static(),
            handlers::project_integration_get,
        );
        map.register(
            OriginProjectIntegrationRequest::descriptor_static(),
            handlers::origin_project_integration_request,
        );
        map.register(
            OriginPackageCreate::descriptor_static(),
            handlers::origin_package_create,
        );
        map.register(
            OriginPackageGet::descriptor_static(),
            handlers::origin_package_get,
        );
        map.register(
            OriginPackageLatestGet::descriptor_static(),
            handlers::origin_package_latest_get,
        );
        map.register(
            OriginPackageListRequest::descriptor_static(),
            handlers::origin_package_list,
        );
        map.register(
            OriginPackagePlatformListRequest::descriptor_static(),
            handlers::origin_package_platform_list,
        );
        map.register(
            OriginPackageChannelListRequest::descriptor_static(),
            handlers::origin_package_channel_list,
        );
        map.register(
            OriginPackageVersionListRequest::descriptor_static(),
            handlers::origin_package_version_list,
        );
        map.register(
            OriginPackageDemote::descriptor_static(),
            handlers::origin_package_demote,
        );
        map.register(
            OriginPackageGroupPromote::descriptor_static(),
            handlers::origin_package_group_promote,
        );
        map.register(
            OriginPackageGroupDemote::descriptor_static(),
            handlers::origin_package_group_demote,
        );
        map.register(
            OriginPackagePromote::descriptor_static(),
            handlers::origin_package_promote,
        );
        map.register(
            OriginPackageUniqueListRequest::descriptor_static(),
            handlers::origin_package_unique_list,
        );
        map.register(
            OriginPackageSearchRequest::descriptor_static(),
            handlers::origin_package_search,
        );
        map.register(
            OriginSecretCreate::descriptor_static(),
            handlers::origin_secret_create,
        );
        map.register(
            OriginSecretListGet::descriptor_static(),
            handlers::origin_secret_list,
        );
        map.register(
            OriginSecretDelete::descriptor_static(),
            handlers::origin_secret_delete,
        );
        map.register(
            OriginChannelCreate::descriptor_static(),
            handlers::origin_channel_create,
        );
        map.register(
            OriginChannelDelete::descriptor_static(),
            handlers::origin_channel_delete,
        );
        map.register(
            OriginChannelGet::descriptor_static(),
            handlers::origin_channel_get,
        );
        map.register(
            OriginChannelListRequest::descriptor_static(),
            handlers::origin_channel_list,
        );
        map.register(
            OriginChannelPackageGet::descriptor_static(),
            handlers::origin_channel_package_get,
        );
        map.register(
            OriginChannelPackageLatestGet::descriptor_static(),
            handlers::origin_channel_package_latest_get,
        );
        map.register(
            OriginChannelPackageListRequest::descriptor_static(),
            handlers::origin_channel_package_list,
        );
        map.register(
            OriginMemberRemove::descriptor_static(),
            handlers::origin_member_delete,
        );
        map.register(MyOriginsRequest::descriptor_static(), handlers::my_origins);
        map.register(
            PackageChannelAudit::descriptor_static(),
            handlers::package_channel_audit,
        );
        map.register(
            PackageGroupChannelAudit::descriptor_static(),
            handlers::package_group_channel_audit,
        );
        // OLD SESSIONSRV HANDLERS
        map.register(
            AccountGet::descriptor_static(),
            session_handlers::account_get,
        );
        map.register(
            AccountGetId::descriptor_static(),
            session_handlers::account_get_id,
        );
        map.register(
            AccountCreate::descriptor_static(),
            session_handlers::account_create,
        );
        map.register(
            AccountUpdate::descriptor_static(),
            session_handlers::account_update,
        );
        map.register(
            AccountFindOrCreate::descriptor_static(),
            session_handlers::account_find_or_create,
        );
        map.register(
            AccountTokenCreate::descriptor_static(),
            session_handlers::account_token_create,
        );
        map.register(
            AccountTokenRevoke::descriptor_static(),
            session_handlers::account_token_revoke,
        );
        map.register(
            AccountTokensGet::descriptor_static(),
            session_handlers::account_tokens_get,
        );
        map.register(
            SessionCreate::descriptor_static(),
            session_handlers::session_create,
        );
        map.register(
            SessionGet::descriptor_static(),
            session_handlers::session_get,
        );
        map
    };
}

#[derive(Clone)]
pub struct ServerState {
    datastore: DataStore,
    sessions: Arc<Box<RwLock<HashSet<Session>>>>,
}

impl ServerState {
    fn new(cfg: Config) -> SrvResult<Self> {
        let datastore = DataStore::new(&cfg.datastore)?;

        Ok(ServerState {
            datastore: datastore,
            sessions: Arc::new(Box::new(RwLock::new(HashSet::default()))),
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

struct OriginSrv;
impl Dispatcher for OriginSrv {
    const APP_NAME: &'static str = "builder-originsrv";
    const PROTOCOL: Protocol = Protocol::OriginSrv;

    type Config = Config;
    type Error = SrvError;
    type State = ServerState;

    fn app_init(
        config: Self::Config,
        _router_pipe: Arc<String>,
    ) -> SrvResult<<Self::State as AppState>::InitState> {
        let state = ServerState::new(config)?;
        Ok(state)
    }

    fn dispatch_table() -> &'static DispatchTable<Self> {
        &DISPATCH_TABLE
    }
}

pub fn run(config: Config) -> AppResult<(), SrvError> {
    app_start::<OriginSrv>(config)
}

pub fn migrate(config: Config) -> SrvResult<()> {
    let ds = DataStore::new(&config.datastore)?;
    ds.setup()
}
