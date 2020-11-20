use crate::bldr_notify::{error::Error,
                         server};
use habitat_builder_notify as bldr_notify;
use std::path::PathBuf;
use structopt::StructOpt;

#[macro_use]
extern crate env_logger;

#[macro_use]
extern crate log;

const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/VERSION"));

#[derive(StructOpt, Debug)]
#[structopt(name = "bldr-notify", version = VERSION, about = "Builder Notifications Service")]
enum Notify {
    /// Run the service
    Run {
        /// Read config.toml at this path
        #[structopt(short, long, parse(from_os_str))]
        config: Option<PathBuf>,
    },
}

fn main() {
    env_logger::init();
    if let Err(e) = start() {
        error!("{}", e);
        std::process::exit(1)
    }
}

fn start() -> Result<(), Error> {
    let opt = Notify::from_args();
    match opt {
        Notify::Run { config } => server::run(config)?,
    }
    Ok(())
}
