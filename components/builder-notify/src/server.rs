use crate::{config::Config,
            error::Error};
use builder_core::config::ConfigFile;
use habitat_builder_events::connection::{create_consumer,
                                         EventConsumer};
use habitat_core::ok_warn;
use std::path::PathBuf;

const CFG_DEFAULT_PATH: &str = "/hab/svc/builder-notify/config/config.toml";
const DEFAULT_QUEUE_NAME: &str = "builder-events";

pub struct AppState {
    event_consumer: Option<Box<dyn EventConsumer>>,
}

impl AppState {
    pub fn new(config: &Config) -> Result<AppState, Error> {
        let event_consumer = ok_warn!(create_consumer(&config.eventbus));

        Ok(AppState { event_consumer })
    }
}

pub async fn run(path: Option<PathBuf>) -> Result<(), Error> {
    info!("Launching builder-notify service.");
    let config_path = path.unwrap_or_else(|| PathBuf::from(CFG_DEFAULT_PATH));
    let config = if config_path.is_file() {
        Config::from_file(config_path).map_err(|e| Error::NotificationsError(Box::new(e)))?
    } else {
        warn!("EventConsumer config file not found at {:?}. Using defaults..",
              config_path);
        Config::default()
    };
    match AppState::new(&config) {
        Ok(state) => {
            if let Some(bus) = state.event_consumer {
                info!("EventConsumer started.");
                bus.subscribe(&[DEFAULT_QUEUE_NAME].to_vec())
                   .map_err(|e| Error::NotificationsError(Box::new(e)))?;

                loop {
                    if let Some(msg) = bus.recv().await {
                        match msg {
                            Ok(builder_event) => {
                                let (event, _) = builder_event.fields();
                                let data: serde_json::Value =
                                    event.try_get_data().unwrap().unwrap();
                                debug!("EventData {:?}", data);
                            }
                            Err(err) => error!("{}", err),
                        }
                    };
                }
            }
        }
        Err(e) => error!("EventConsumer failed to start: {}", e),
    }
    Ok(())
}
