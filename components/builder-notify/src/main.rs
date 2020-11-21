use habitat_builder_notify::{cli::Notify,
                             error::Error,
                             server};
use structopt::StructOpt;

#[macro_use]
extern crate log;
extern crate env_logger;

fn main() -> Result<(), Error> {
    env_logger::init();
    match Notify::from_args() {
        Notify::Run { config } => {
            if let Err(e) = server::run(config) {
                error!("{}", e);
                std::process::exit(1)
            }
        }
    }
    Ok(())
}
