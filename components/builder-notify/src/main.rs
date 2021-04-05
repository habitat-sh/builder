use habitat_builder_notify::{cli::Notify,
                             server};
use structopt::StructOpt;

#[macro_use]
extern crate log;
extern crate env_logger;

#[actix_rt::main]
async fn main() {
    env_logger::init();
    match Notify::from_args() {
        Notify::Run { config } => {
            if let Err(e) = server::run(config).await {
                error!("Error in 'builder-notify' service {}", e);
                std::process::exit(1)
            }
        }
    }
}
