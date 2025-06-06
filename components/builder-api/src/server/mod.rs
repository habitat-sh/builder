pub mod authorize;
pub mod error;
pub mod framework;
pub mod helpers;
pub mod migrations;
pub mod provision;
pub mod resources;
pub mod services;

use self::{framework::middleware::authentication_middleware,
           resources::{authenticate::Authenticate,
                       channels::Channels,
                       events::Events,
                       ext::Ext,
                       origins::Origins,
                       pkgs::Packages,
                       profile::Profile,
                       settings::Settings,
                       user::User},
           services::{memcache::MemcacheClient,
                      s3::S3Handler}};
use crate::{bldr_core::keys,
            config::{Config,
                     GatewayCfg},
            db::{migration,
                 DbPool}};
use actix_web::{http::{KeepAlive,
                       StatusCode},
                middleware::Logger,
                web,
                App,
                HttpResponse,
                HttpServer};
use artifactory_client::client::ArtifactoryClient;
use oauth_client::client::OAuth2Client;
use openssl::ssl::{SslAcceptor,
                   SslFiletype,
                   SslMethod,
                   SslVerifyMode};
use rand::{self,
           Rng};
use resources::jobs::Jobs;
use std::{cell::RefCell,
          collections::HashMap,
          iter::FromIterator,
          sync::Arc,
          time::Duration};

// This cipher list corresponds to the "intermediate" configuration
// recommended by Mozilla:
//
// https://ssl-config.mozilla.org/#server=nginx&server-version=1.17.0&config=intermediate&hsts=false&ocsp=false
//
// TODO(ssd) 2019-11-08: We can remove this when we upgrade the
// openssl create to a version that includes mozilla_intermediate_v5:
//
// https://github.com/sfackler/rust-openssl/commit/1b3e0c8a15f11f07b076f1b83278d5ec99881ff1
const TLS_CIPHERS: &str = "ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:\
                           ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:\
                           ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:\
                           DHE-RSA-AES128-GCM-SHA256:DHE-RSA-AES256-GCM-SHA384";

features! {
    pub mod feat {
        const List = 0b0000_0001,
        const LegacyProject = 0b0000_0011,
        const Artifactory = 0b0000_0100,
        const BuildDeps = 0b0000_1000
    }
}

// Application state
pub struct AppState {
    config:      Config,
    packages:    S3Handler,
    oauth:       OAuth2Client,
    memcache:    RefCell<MemcacheClient>,
    artifactory: ArtifactoryClient,
    db:          DbPool,
}

impl AppState {
    pub fn new(config: &Config, db: DbPool) -> error::Result<AppState> {
        let app_state =
            AppState { config: config.clone(),
                       packages: S3Handler::new(config.s3.clone()),
                       oauth: OAuth2Client::new(config.oauth.clone())?,
                       memcache: RefCell::new(MemcacheClient::new(&config.memcache.clone())),
                       artifactory: ArtifactoryClient::new(config.artifactory.clone())?,
                       db };

        Ok(app_state)
    }
}

fn enable_features(config: &Config) {
    let features: HashMap<_, _> = HashMap::from_iter(vec![("LIST", feat::List),
                                                          ("LEGACYPROJECT", feat::LegacyProject),
                                                          ("ARTIFACTORY", feat::Artifactory),
                                                          ("BUILDDEPS", feat::BuildDeps),]);

    for key in &config.api.features_enabled {
        if features.contains_key(key.as_str()) {
            info!("Enabling feature: {}", key);
            if let Some(flag) = features.get(key.as_str()) {
                feat::enable(*flag);
            }
        }
    }

    if feat::is_enabled(feat::List) {
        println!("Listing possible feature flags: {:?}", features.keys());
        println!("Enable features by populating 'features_enabled' in config");
    }
}

/// Endpoint for determining availability of builder-api components.
///
/// Returns a status 200 on success. Any non-200 responses are an outage or a partial outage.
pub async fn status() -> HttpResponse { HttpResponse::new(StatusCode::OK) }

pub async fn run(config: Config) -> error::Result<()> {
    enable_features(&config);

    let cfg = Arc::new(config.clone());
    let db_pool = DbPool::new(&config.datastore.clone());

    // Check if the builder encryption key is present; if not, panic with an appropriate error.
    if let Err(e) = keys::get_latest_builder_key(&config.api.key_path) {
        panic!("Failed to get the builder encryption key, error = {}", e);
    }
    let mut conn = db_pool.get_conn().unwrap();
    migration::setup(&mut conn).unwrap();
    migrations::migrate_to_encrypted(&mut conn, &config.api.key_path).unwrap();

    migrations::encrypt_secret_keys::run(&mut conn, &config.api.key_path)
        .expect("Error encrypting secret keys");

    // Bootstrap the user if automatic provisioning of the account is enabled.
    if config.provision.auto_provision_account {
        info!("bootstrapping user");
        let app_state = match AppState::new(&config, db_pool.clone()) {
            Ok(state) => state,
            Err(err) => {
                error!("Unable to create application state, err = {}", err);
                panic!("Cannot start without valid application state");
            }
        };

        match provision::provision_bldr_environment(&app_state) {
            Ok(_) => {
                info!("Token has been successfully provisioned and stored.");
            }
            Err(e) => {
                error!("Error during bldr account provisioning, err = {}", e);
                panic!("Error during bldr account provisioning, err = {}", e);
            }
        }
    }

    let mut srv = HttpServer::new(move || {
                      let app_state = match AppState::new(&config, db_pool.clone()) {
                          Ok(state) => state,
                          Err(err) => {
                              error!("Unable to create application state, err = {}", err);
                              panic!("Cannot start without valid application state");
                          }
                      };

                      App::new()
            .app_data(web::Data::new(app_state))
            .wrap_fn(authentication_middleware)
            .wrap(Logger::default().exclude("/v1/status"))
            .service(
                web::scope("/v1")
                    .configure(Authenticate::register)
                    .configure(Channels::register)
                    .configure(Ext::register)
                    .configure(Jobs::register)
                    .configure(Origins::register)
                    .configure(Packages::register)
                    .configure(Profile::register)
                    .configure(Settings::register)
                    .configure(User::register)
                    .configure(Events::register)
                    .service(
                        web::resource("/status")
                            .route(web::get().to(status))
                            .route(web::head().to(status)),
                    ),
            )
                  }).workers(cfg.handler_count())
                    .keep_alive(KeepAlive::from(Duration::from_secs(cfg.http.keep_alive as u64)));

    info!("builder-api listening on {}:{}",
          cfg.listen_addr(),
          cfg.listen_port());

    srv = match &cfg.http.tls {
        Some(tls_cfg) => {
            info!("TLS enabled (key: {:?}, cert: {:?})",
                  tls_cfg.key_path, tls_cfg.cert_path);
            let mut builder = SslAcceptor::mozilla_modern(SslMethod::tls())?;
            builder.set_private_key_file(&tls_cfg.key_path, SslFiletype::PEM)?;
            builder.set_certificate_chain_file(&tls_cfg.cert_path)?;
            builder.set_cipher_list(TLS_CIPHERS)?;
            let random_bytes = rand::thread_rng().gen::<[u8; 16]>();
            builder.set_session_id_context(&random_bytes)?;

            match &tls_cfg.ca_cert_path {
                None => {
                    info!("TLS client authentication disabled");
                }
                Some(ca_cert_path) => {
                    info!("TLS client authentication enabled");
                    let mut verify_mode = SslVerifyMode::empty();
                    verify_mode.insert(SslVerifyMode::PEER);
                    verify_mode.insert(SslVerifyMode::FAIL_IF_NO_PEER_CERT);
                    builder.set_verify(verify_mode);
                    builder.set_ca_file(ca_cert_path)?;
                }
            }

            srv.bind_openssl(cfg.http.clone(), builder)?
        }
        None => srv.bind(cfg.http.clone())?,
    };
    Ok(srv.run().await?)
}

impl Clone for feat::Flags {
    fn clone(&self) -> Self { *self }
}

impl Copy for feat::Flags {}
impl std::fmt::Debug for feat::Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Flags({:#010b})", self.bits())
    }
}
