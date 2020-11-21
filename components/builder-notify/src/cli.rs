use std::path::PathBuf;
use structopt::StructOpt;

const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/VERSION"));

#[derive(StructOpt, Debug)]
#[structopt(name = "bldr-notify", version = VERSION, about = "Builder Notifications Service")]
pub enum Notify {
    /// Run the service
    Run {
        /// Read config.toml at this path
        #[structopt(short, long, parse(from_os_str))]
        config: Option<PathBuf>,
    },
}
