use std;
use std::fmt::{self, Display};

use serde::{de, ser};

pub type Result<T> = std::result::Result<T, Error>;

// This is a bare-bones implementation. A real library would provide additional
// information in its error type, for example the line and column at which the
// error occurred, the byte offset into the input, or the current key being
// processed.
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
  // One or more variants that can be created by data structures through the
  // `ser::Error` and `de::Error` traits. For example the Serialize impl for
  // Mutex<T> might return an error because the mutex is poisoned, or the
  // Deserialize impl for a struct may return an error because a required
  // field is missing.
  Message(String),

  // Zero or more variants that can be created directly by the Serializer and
  // Deserializer without going through `ser::Error` and `de::Error`. These
  // are specific to the format, in this case JSON.
  Eof,
  NoUnitType,
}

impl ser::Error for Error {
  fn custom<T: Display>(msg: T) -> Self {
    Error::Message(msg.to_string())
  }
}

impl de::Error for Error {
  fn custom<T: Display>(msg: T) -> Self {
    Error::Message(msg.to_string())
  }
}

impl Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Error::Message(msg) => write!(f, "{}", msg),
      Error::Eof => write!(f, "unexpected end of input"),
      Error::NoUnitType => write!(f, "there is no none type in NBT"),
    }
  }
}

impl std::error::Error for Error {}
