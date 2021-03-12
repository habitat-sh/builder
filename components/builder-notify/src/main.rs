use habitat_builder_notify::{cli::Notify,
                             server};
use std::{fmt,
          process};
use structopt::StructOpt;

extern crate env_logger;

#[actix_rt::main]
async fn main() {
    env_logger::init();
    match Notify::from_args() {
        Notify::Run { config } => {
            match server::run(config).await {
                Ok(_) => std::process::exit(0),
                Err(e) => exit_with(e, 1),
            }
        }
    }
}

fn exit_with<T>(err: T, code: i32)
    where T: fmt::Display
{
    println!("Error in 'builder-notify' service {}", err);
    process::exit(code)
}
