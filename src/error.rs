//! Error and Result module.
use std::error::Error as StdError;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io::Error as IoError;
use std::str::Utf8Error;
use std::string::FromUtf8Error;

use httparse;
use url;

#[cfg(feature = "openssl")]
use openssl::ssl::error::SslError;

use self::Error::{
    Method,
    Uri,
    Version,
    Header,
    Status,
    Io,
    Ssl,
    TooLarge,
    Utf8
};

pub use url::ParseError;

/// Result type often returned from methods that can have mco_http `Error`s.
pub type Result<T> = ::std::result::Result<T, Error>;

/// A set of errors that can occur parsing HTTP streams.
#[derive(Debug)]
pub enum Error {
    /// An invalid `Method`, such as `GE,T`.
    Method,
    /// An invalid `RequestUri`, such as `exam ple.domain`.
    Uri(url::ParseError),
    /// An invalid `HttpVersion`, such as `HTP/1.1`
    Version,
    /// An invalid `Header`.
    Header,
    /// A message head is too large to be reasonable.
    TooLarge,
    /// An invalid `Status`, such as `1337 ELITE`.
    Status,
    /// An `io::Error` that occurred while trying to read or write to a network stream.
    Io(IoError),
    /// An error from a SSL library.
    Ssl(Box<dyn StdError + Send + Sync>),
    /// Parsing a field as string failed
    Utf8(Utf8Error),

    /// Other error
    Other(String),

    #[doc(hidden)]
    __Nonexhaustive(Void)
}

#[doc(hidden)]
pub struct Void(());

impl fmt::Debug for Void {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        unreachable!()
    }
}

impl StdError for Error {
    fn cause(&self) -> Option<&dyn StdError> {
        match *self {
            Io(ref error) => Some(error),
            Ssl(ref error) => Some(&**error),
            Uri(ref error) => Some(error),
            Utf8(ref error) => Some(error),
            _ => None,
        }
    }
}

impl Display for Error{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Method => f.write_str("Invalid Method specified"),
            Version => f.write_str("Invalid HTTP version specified"),
            Header => f.write_str("Invalid Header provided"),
            TooLarge => f.write_str("Message head is too large"),
            Status => f.write_str("Invalid Status provided"),
            Uri(e) => write!(f, "{}", e),
            Io(e) => write!(f, "{}", e),
            Ssl(e) => write!(f, "{}", e),
            Utf8(e) => write!(f, "{}", e),
            Error::Other(e) => write!(f, "{}", e),
            Error::__Nonexhaustive(..) =>  unreachable!(),
        }
    }
}

impl From<IoError> for Error {
    fn from(err: IoError) -> Error {
        Io(err)
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Error {
        Uri(err)
    }
}

#[cfg(feature = "openssl")]
impl From<SslError> for Error {
    fn from(err: SslError) -> Error {
        match err {
            SslError::StreamError(err) => Io(err),
            err => Ssl(Box::new(err)),
        }
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Error {
        Utf8(err)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Error {
        Utf8(err.utf8_error())
    }
}

impl From<httparse::Error> for Error {
    fn from(err: httparse::Error) -> Error {
        match err {
            httparse::Error::HeaderName => Header,
            httparse::Error::HeaderValue => Header,
            httparse::Error::NewLine => Header,
            httparse::Error::Status => Status,
            httparse::Error::Token => Header,
            httparse::Error::TooManyHeaders => TooLarge,
            httparse::Error::Version => Version,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::io;
    use httparse;
    use url;
    use super::Error;
    use super::Error::*;

    #[test]
    fn test_cause() {
        let orig = io::Error::new(io::ErrorKind::Other, "other");
        let desc = orig.description().to_owned();
        let e = Io(orig);
        assert_eq!(e.cause().unwrap().description(), desc);
    }

    macro_rules! from {
        ($from:expr => $error:pat) => {
            match Error::from($from) {
                e @ $error => {
                    assert!(e.description().len() > 5);
                } ,
                _ => panic!("{:?}", $from)
            }
        }
    }

    macro_rules! from_and_cause {
        ($from:expr => $error:pat) => {
            match Error::from($from) {
                e @ $error => {
                    let desc = e.cause().unwrap().to_string();
                    assert_eq!(desc, $from.to_string().to_owned());
                    assert_eq!(desc, e.to_string());
                },
                _ => panic!("{:?}", $from)
            }
        }
    }

    #[test]
    fn test_from() {

        from_and_cause!(io::Error::new(io::ErrorKind::Other, "other") => Io(..));
        from_and_cause!(url::ParseError::EmptyHost => Uri(..));

        from!(httparse::Error::HeaderName => Header);
        from!(httparse::Error::HeaderName => Header);
        from!(httparse::Error::HeaderValue => Header);
        from!(httparse::Error::NewLine => Header);
        from!(httparse::Error::Status => Status);
        from!(httparse::Error::Token => Header);
        from!(httparse::Error::TooManyHeaders => TooLarge);
        from!(httparse::Error::Version => Version);
    }

    #[cfg(feature = "openssl")]
    #[test]
    fn test_from_ssl() {
        use openssl::ssl::error::SslError;

        from!(SslError::StreamError(
            io::Error::new(io::ErrorKind::Other, "ssl negotiation")) => Io(..));
        from_and_cause!(SslError::SslSessionClosed => Ssl(..));
    }
}
