use std::{error,
          fmt,
          result};

#[derive(Debug)]
pub enum Error {
    BuilderCore(builder_core::Error),
    NotificationsError(Box<dyn std::error::Error>),
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            Error::BuilderCore(ref e) => format!("{}", e),
            Error::NotificationsError(ref e) => e.to_string(),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for Error {}

impl From<builder_core::Error> for Error {
    fn from(err: builder_core::Error) -> Error { Error::BuilderCore(err) }
}
