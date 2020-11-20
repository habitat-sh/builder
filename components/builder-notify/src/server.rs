use crate::{bldr_events::connection::{create_consumer,
                                      EventBusConsumer},
            config::Config,
            error::Error,
            hab_core::{config::ConfigFile,
                       ok_warn}};
use std::{path::PathBuf,
          thread::{self},
          time::Duration};

const CFG_DEFAULT_PATH: &str = "/hab/svc/builder-notify/config/config.toml";
const DEFAULT_QUEUE_NAME: &str = "builder_events";

pub struct AppState {
    eventconsumer: Option<Box<dyn EventBusConsumer>>,
}

impl AppState {
    pub fn new(config: &Config) -> Result<AppState, Error> {
        let mut app_state = AppState { eventconsumer: None, };

        app_state.eventconsumer = ok_warn!(create_consumer(&config.eventbus));

        Ok(app_state)
    }
}

pub fn run(path: Option<PathBuf>) -> Result<(), Error> {
    info!("Launching builder-notify service.");
    let config_path = path.unwrap_or_else(|| PathBuf::from(CFG_DEFAULT_PATH));
    let config = if config_path.is_file() {
        Config::from_file(config_path).map_err(|e| Error::NotificationsError(Box::new(e)))?
    } else {
        warn!("EventBusConsumer config file not found at {:?}. Using defaults..",
              config_path);
        Config::default()
    };
    match AppState::new(&config) {
        Ok(state) => {
            if let Some(bus) = state.eventconsumer {
                info!("EventBusConsumer started.");
                bus.subscribe([DEFAULT_QUEUE_NAME].to_vec())
                   .map_err(|e| Error::NotificationsError(Box::new(e)))?;

                loop {
                    if let Some(msg) = bus.poll() {
                        debug!("received msg {:?}", msg);
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        }
        Err(e) => error!("EventBusConsumer failed to start: {}", e),
    }
    Ok(())
}
