//! Pieces pertaining to the HTTP message protocol.
use std::borrow::Cow;
use crate::proto;
use crate::version::HttpVersion;
use crate::version::HttpVersion::{Http10, Http11};


pub use self::message::{HttpMessage, RequestHead, ResponseHead, Protocol};

pub mod h1;
pub mod message;

/// The raw status code and reason-phrase.

// pub struct RawStatus(pub u16, pub Cow<'static, str>);
pub type   RawStatus= http::StatusCode;

/// Checks if a connection should be kept alive.
#[inline]
pub fn should_keep_alive(version: http::Version, headers: &http::HeaderMap) -> bool {
    trace!("should_keep_alive( {:?}, {:?} )", version, headers.get(http::header::CONNECTION));
    match (version, headers.get(http::header::CONNECTION)) {
        (http::Version::HTTP_10, None) => false,
        (http::Version::HTTP_10, Some(conn)) if !conn.to_str().unwrap_or_default().contains("keep-alive") => false,
        (http::Version::HTTP_11, Some(conn)) if conn.to_str().unwrap_or_default().contains("close")  => false,
        _ => true
    }
}

#[test]
fn test_should_keep_alive() {
    let mut headers = Headers::new();

    assert!(!should_keep_alive(Http10, &headers));
    assert!(should_keep_alive(Http11, &headers));

    headers.set(Connection::close());
    assert!(!should_keep_alive(Http10, &headers));
    assert!(!should_keep_alive(Http11, &headers));

    headers.set(Connection::keep_alive());
    assert!(should_keep_alive(Http10, &headers));
    assert!(should_keep_alive(Http11, &headers));
}
