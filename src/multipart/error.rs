// Copyright 2016 mime-multipart Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::io;
use std::string::FromUtf8Error;
use httparse;

/// An error type for the `mime-multipart` crate.
pub enum Error {
    /// The mco_http request did not have a Content-Type header.
    NoRequestContentType,
    /// The mco_http request Content-Type top-level Mime was not `Multipart`.
    NotMultipart,
    /// The Content-Type header failed to specify boundary token.
    BoundaryNotSpecified,
    /// A multipart section contained only partial headers.
    PartialHeaders,
    EofInMainHeaders,
    EofBeforeFirstBoundary,
    NoCrLfAfterBoundary,
    EofInPartHeaders,
    EofInFile,
    EofInPart,
    /// An HTTP parsing error from a multipart section.
    Httparse(httparse::Error),
    /// An I/O error.
    Io(io::Error),
    /// An error was returned from Hyper.
    Hyper(crate::Error),
    /// An error occurred during UTF-8 processing.
    Utf8(FromUtf8Error),
    /// An error occurred during character decoding
    Decoding(Cow<'static, str>),

    MissingDisposition,
    NoName
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<httparse::Error> for Error {
    fn from(err: httparse::Error) -> Error {
        Error::Httparse(err)
    }
}

impl From<crate::Error> for Error {
    fn from(err: crate::Error) -> Error {
        Error::Hyper(err)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Error {
        Error::Utf8(err)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Httparse(ref e) =>
                std::fmt::Display::fmt(&e, f),
            Error::Io(ref e) =>
                std::fmt::Display::fmt(&e, f),
            Error::Hyper(ref e) =>
                std::fmt::Display::fmt(&e, f),
            Error::Utf8(ref e) =>
                std::fmt::Display::fmt(&e, f),
            Error::Decoding(ref e) =>
                std::fmt::Display::fmt(&e, f),

            Error::NoRequestContentType => {
                f.write_str("NoRequestContentType")
            }
            Error::NotMultipart => {
                f.write_str("NotMultipart")
            }
            Error::BoundaryNotSpecified => {
                f.write_str("BoundaryNotSpecified")
            }
            Error::PartialHeaders => {
                f.write_str("PartialHeaders")
            }
            Error::EofInMainHeaders => {
                f.write_str("EofInMainHeaders")
            }
            Error::EofBeforeFirstBoundary => {
                f.write_str("EofBeforeFirstBoundary")
            }
            Error::NoCrLfAfterBoundary => {
                f.write_str("NoCrLfAfterBoundary")
            }
            Error::EofInPartHeaders => {
                f.write_str("EofInPartHeaders")
            }
            Error::EofInFile => {
                f.write_str("EofInFile")
            }
            Error::EofInPart => {
                f.write_str("EofInPart")
            }
            Error::MissingDisposition => {
                f.write_str("MissingDisposition")
            }
            Error::NoName => {
                f.write_str("NoName")
            }
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)?;
        if self.source().is_some() {
            write!(f, ": {:?}", self.source().unwrap())?; // recurse
        }
        Ok(())
    }
}

impl StdError for Error {
    fn description(&self) -> &str{
        match *self {
            Error::NoRequestContentType => "The mco_http request did not have a Content-Type header.",
            Error::NotMultipart =>
                "The mco_http request Content-Type top-level Mime was not multipart.",
            Error::BoundaryNotSpecified =>
                "The Content-Type header failed to specify a boundary token.",
            Error::PartialHeaders =>
                "A multipart section contained only partial headers.",
            Error::EofInMainHeaders =>
                "The request headers ended pre-maturely.",
            Error::EofBeforeFirstBoundary =>
                "The request body ended prior to reaching the expected starting boundary.",
            Error::NoCrLfAfterBoundary =>
                "Missing CRLF after boundary.",
            Error::EofInPartHeaders =>
                "The request body ended prematurely while parsing headers of a multipart part.",
            Error::EofInFile =>
                "The request body ended prematurely while streaming a file part.",
            Error::EofInPart =>
                "The request body ended prematurely while reading a multipart part.",
            Error::Httparse(_) =>
                "A parse error occurred while parsing the headers of a multipart section.",
            Error::Io(_) => "An I/O error occurred.",
            Error::Hyper(_) => "A mco_http error occurred.",
            Error::Utf8(_) => "A UTF-8 error occurred.",
            Error::Decoding(_) => "A decoding error occurred.",
            Error::MissingDisposition => "MissingDisposition",
            Error::NoName => "no name",
        }
    }
}
