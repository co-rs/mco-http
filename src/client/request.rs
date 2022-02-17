//! Client Requests
use std::marker::PhantomData;
use std::io::{self, Write};

use std::time::Duration;
use http::uri::Scheme;

use url::Url;

use crate::method::Method;
use crate::net::{NetworkStream, NetworkConnector, DefaultConnector, Fresh, Streaming};
use crate::{header_value, version};
use crate::client::{Response, get_host_and_port};

use crate::proto::{HttpMessage, RequestHead};
use crate::proto::h1::Http11Message;
use http::HeaderValue;

/// A client request to a remote server.
/// The W type tracks the state of the request, Fresh vs Streaming.
pub struct Request<W> {
    /// The target URI for this request.
    pub url: http::Uri,

    /// The HTTP version of this request.
    pub version: http::Version,

    message: Box<dyn HttpMessage>,
    headers: http::HeaderMap,
    method: http::Method,

    _marker: PhantomData<W>,
}

impl<W> Request<W> {
    /// Read the Request headers.
    #[inline]
    pub fn headers(&self) -> &http::HeaderMap { &self.headers }

    /// Read the Request method.
    #[inline]
    pub fn method(&self) -> http::Method { self.method.clone() }

    /// Set the write timeout.
    #[inline]
    pub fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.message.set_write_timeout(dur)
    }

    /// Set the read timeout.
    #[inline]
    pub fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.message.set_read_timeout(dur)
    }
}

impl Request<Fresh> {
    /// Create a new `Request<Fresh>` that will use the given `HttpMessage` for its communication
    /// with the server. This implies that the given `HttpMessage` instance has already been
    /// properly initialized by the caller (e.g. a TCP connection's already established).
    pub fn with_message(method: http::Method, url: http::Uri, message: Box<dyn HttpMessage>)
                        -> crate::Result<Request<Fresh>> {
        let mut headers = http::HeaderMap::with_capacity(1);
        {
            let (host, port) = r#try!(get_host_and_port(&url));
            if port == 0 || port == 80 || port == 443 {
                headers.insert(http::header::HOST, header_value!(&format!("{}:{}",host,port)));
            } else {
                headers.insert(http::header::HOST, header_value!(host));
            }
        }

        Ok(Request::with_headers_and_message(method, url, headers, message))
    }

    #[doc(hidden)]
    pub fn with_headers_and_message(method: http::Method, url: http::Uri, headers: http::HeaderMap, message: Box<dyn HttpMessage>)
                                    -> Request<Fresh> {
        Request {
            method: method,
            headers: headers,
            url: url,
            version: http::Version::HTTP_11,
            message: message,
            _marker: PhantomData,
        }
    }

    /// Create a new client request.
    pub fn new(method: http::Method, url: http::Uri) -> crate::Result<Request<Fresh>> {
        let conn = DefaultConnector::default();
        Request::with_connector(method, url, &conn)
    }

    /// Create a new client request with a specific underlying NetworkStream.
    pub fn with_connector<C, S>(method: http::Method, url: http::Uri, connector: &C)
                                -> crate::Result<Request<Fresh>> where
        C: NetworkConnector<Stream=S>,
        S: Into<Box<dyn NetworkStream + Send>> {
        let stream = {
            let (host, port) = r#try!(get_host_and_port(&url));
            r#try!(connector.connect(host, port, { match url.scheme().as_ref(){
            None => {""}
            Some(s) => {s.as_str()}
        }})).into()
        };

        Request::with_message(method, url, Box::new(Http11Message::with_stream(stream)))
    }

    /// Consume a Fresh Request, writing the headers and method,
    /// returning a Streaming Request.
    pub fn start(mut self) -> crate::Result<Request<Streaming>> {
        let head = match self.message.set_outgoing(RequestHead {
            headers: self.headers,
            method: self.method,
            url: self.url,
        }) {
            Ok(head) => head,
            Err(e) => {
                let _ = self.message.close_connection();
                return Err(From::from(e));
            }
        };

        Ok(Request {
            method: head.method,
            headers: head.headers,
            url: head.url,
            version: self.version,
            message: self.message,
            _marker: PhantomData,
        })
    }

    /// Get a mutable reference to the Request headers.
    #[inline]
    pub fn headers_mut(&mut self) -> &mut http::HeaderMap { &mut self.headers }
}


impl Request<Streaming> {
    /// Completes writing the request, and returns a response to read from.
    ///
    /// Consumes the Request.
    pub fn send(self) -> crate::Result<Response> {
        Response::with_message(self.url, self.message)
    }
}

impl Write for Request<Streaming> {
    #[inline]
    fn write(&mut self, msg: &[u8]) -> io::Result<usize> {
        match self.message.write(msg) {
            Ok(n) => Ok(n),
            Err(e) => {
                let _ = self.message.close_connection();
                Err(e)
            }
        }
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        match self.message.flush() {
            Ok(r) => Ok(r),
            Err(e) => {
                let _ = self.message.close_connection();
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::str::{from_utf8, FromStr};
    use http::header::CONTENT_LENGTH;
    use http::Uri;
    use crate::method::Method::{Get, Head, Post};
    use crate::mock::{MockStream, MockConnector};
    use crate::net::Fresh;
    use url::form_urlencoded;
    use crate::header_value;
    use super::Request;
    use crate::proto::h1::Http11Message;

    fn run_request(req: Request<Fresh>) -> Vec<u8> {
        let req = req.start().unwrap();
        let message = req.message;
        let mut message = message.downcast::<Http11Message>().ok().unwrap();
        message.flush_outgoing().unwrap();
        let stream = *message
            .into_inner().downcast::<MockStream>().ok().unwrap();
        stream.write
    }

    fn assert_no_body(s: &str) {
        assert!(!s.contains("Content-Length:"));
        assert!(!s.contains("Transfer-Encoding:"));
    }

    #[test]
    fn test_get_empty_body() {
        let req = Request::with_connector(
            http::Method::GET, http::uri::Uri::from_str("http://example.dom").unwrap(), &mut MockConnector,
        ).unwrap();
        let bytes = run_request(req);
        let s = from_utf8(&bytes[..]).unwrap();
        assert_no_body(s);
    }

    #[test]
    fn test_head_empty_body() {
        let req = Request::with_connector(
            http::Method::HEAD, http::uri::Uri::from_str("http://example.dom").unwrap(), &mut MockConnector,
        ).unwrap();
        let bytes = run_request(req);
        let s = from_utf8(&bytes[..]).unwrap();
        assert_no_body(s);
    }

    #[test]
    fn test_url_query() {
        let url = Uri::from_str("http://example.dom?q=value").unwrap();
        let req = Request::with_connector(
            http::Method::GET, url, &mut MockConnector,
        ).unwrap();
        let bytes = run_request(req);
        let s = from_utf8(&bytes[..]).unwrap();
        assert!(s.contains("?q=value"));
    }

    #[test]
    fn test_post_content_length() {
        let url = Uri::from_str("http://example.dom").unwrap();
        let mut req = Request::with_connector(
            http::Method::POST, url, &mut MockConnector,
        ).unwrap();
        let mut body = String::new();
        form_urlencoded::Serializer::new(&mut body).append_pair("q", "value");
        req.headers_mut().insert(CONTENT_LENGTH,body.len().to_string().parse().unwrap());//.set(ContentLength(body.len() as u64));
        let bytes = run_request(req);
        let s = from_utf8(&bytes[..]).unwrap();
        assert!(s.contains("Content-Length:"));
    }

    #[test]
    fn test_post_chunked() {
        let url = Uri::from_str("http://example.dom").unwrap();
        let req = Request::with_connector(
            http::Method::POST, url, &mut MockConnector,
        ).unwrap();
        let bytes = run_request(req);
        let s = from_utf8(&bytes[..]).unwrap();
        assert!(!s.contains("Content-Length:"));
    }

    #[test]
    fn test_host_header() {
        let url = Uri::from_str("http://example.dom").unwrap();
        let req = Request::with_connector(
            http::Method::GET, url, &mut MockConnector,
        ).unwrap();
        let bytes = run_request(req);
        let s = from_utf8(&bytes[..]).unwrap();
        assert!(s.contains("Host: example.dom"));
    }

    #[test]
    fn test_proxy() {
        let url = Uri::from_str("http://example.dom").unwrap();
        let mut req = Request::with_connector(
            http::Method::GET, url, &mut MockConnector,
        ).unwrap();
        req.message.set_proxied(true);
        let bytes = run_request(req);
        let s = from_utf8(&bytes[..]).unwrap();
        let request_line = "GET http://example.dom/ HTTP/1.1";
        assert_eq!(&s[..request_line.len()], request_line);
        assert!(s.contains("Host: example.dom"));
    }

    #[test]
    fn test_post_chunked_with_encoding() {
        let url = Uri::from_str("http://example.dom").unwrap();
        let mut req = Request::with_connector(
            http::Method::POST, url, &mut MockConnector,
        ).unwrap();
        req.headers_mut().insert(http::header::TRANSFER_ENCODING, "chunked".parse().unwrap());
        let bytes = run_request(req);
        let s = from_utf8(&bytes[..]).unwrap();
        assert!(!s.contains("Content-Length:"));
        assert!(s.contains("Transfer-Encoding:"));
    }

    #[test]
    fn test_write_error_closes() {
        let url = Uri::from_str("http://hyper.rs").unwrap();
        let req = Request::with_connector(
            http::Method::GET, url, &mut MockConnector,
        ).unwrap();
        let mut req = req.start().unwrap();

        req.message.downcast_mut::<Http11Message>().unwrap()
            .get_mut().downcast_mut::<MockStream>().unwrap()
            .error_on_write = true;

        req.write(b"foo").unwrap();
        assert!(req.flush().is_err());

        assert!(req.message.downcast_ref::<Http11Message>().unwrap()
            .get_ref().downcast_ref::<MockStream>().unwrap()
            .is_closed);
    }
}
