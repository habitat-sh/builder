use std::{error,
          fmt,
          result};

#[derive(Debug)]
pub enum Error {
    BadProvider(String),
    EventBusError(Box<dyn std::error::Error>),
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            Error::BadProvider(ref e) => e.to_string(),
            Error::EventBusError(ref e) => e.to_string(),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for Error {}
