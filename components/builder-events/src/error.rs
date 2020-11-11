use cloudevents::event::EventBuilderError;
use std::{error,
          fmt,
          result};

#[derive(Debug)]
pub enum Error {
    BadProvider(String),
    CloudEventBuilderError(EventBuilderError),
    KafkaError(rdkafka::error::KafkaError),
    UrlParseError(url::ParseError),
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            Error::BadProvider(ref e) => e.to_string(),
            Error::CloudEventBuilderError(ref e) => format!("{}", e),
            Error::KafkaError(ref e) => format!("{}", e),
            Error::UrlParseError(ref e) => format!("{}", e),
        };
        write!(f, "{}", msg)
    }
}

impl error::Error for Error {}

impl From<EventBuilderError> for Error {
    fn from(err: EventBuilderError) -> Error { Error::CloudEventBuilderError(err) }
}

impl From<rdkafka::error::KafkaError> for Error {
    fn from(err: rdkafka::error::KafkaError) -> Error { Error::KafkaError(err) }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Error { Error::UrlParseError(err) }
}