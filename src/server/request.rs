//! Server Requests
//!
//! These are requests that a `cogo_http::Server` receives, and include its method,
//! target URI, headers, and message body.
use std::io::{self, Read};
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::time::Duration;
use http::{Extensions, HeaderMap};
use http::request::Parts;

use crate::buffer::BufReader;
use crate::net::NetworkStream;
use crate::version::{HttpVersion};
use crate::method::Method;
use crate::header::{Headers, ContentLength, TransferEncoding, Header};
use crate::proto::h1::{self, Incoming, HttpReader};
use crate::proto::h1::HttpReader::{SizedReader, ChunkedReader, EmptyReader};
use crate::uri::RequestUri;

/// A request bundles several parts of an incoming `NetworkStream`, given to a `Handler`.
pub struct Request<'a, 'b: 'a> {
    pub inner: http::Request<HttpReader<&'a mut BufReader<&'b mut dyn NetworkStream>>>,
}

impl<'a, 'b: 'a> Deref for Request<'a, 'b> {
    type Target = http::Request<HttpReader<&'a mut BufReader<&'b mut dyn NetworkStream>>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, 'b: 'a> DerefMut for Request<'a, 'b> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}


impl<'a, 'b: 'a> Request<'a, 'b> {
    /// Create a new Request, reading the StartLine and Headers so they are
    /// immediately useful.
    pub fn new(stream: &'a mut BufReader<&'b mut dyn NetworkStream>, addr: SocketAddr)
               -> crate::Result<Request<'a, 'b>> {
        let Incoming { version, subject: (method, uri), headers } = r#try!(h1::parse_request(stream));
        debug!("Request Line: {:?} {:?} {:?}", method, uri, version);
        debug!("{:?}", headers);

        let body = if let Some(content_len) = headers.get(http::header::CONTENT_LENGTH) {
            let cl = content_len.to_str().unwrap_or_default().parse()?;
            SizedReader(stream, cl)
        } else if let Some(v) = headers.get(TransferEncoding::header_name()) {
            todo!("check for Transfer-Encoding: chunked");
            ChunkedReader(stream, None)
        } else {
            EmptyReader(stream)
        };


        let mut ext = Extensions::new();
        ext.insert(addr);
        let mut req = http::Request::builder()
            .extension(ext)
            .method(method.as_ref())
            .uri(uri)
            .version(version.into())
            .body(body)
            .unwrap();
        *req.headers_mut() = headers;
        Ok(Self {
            inner: req
        })
    }


    pub fn body(&self) -> &HttpReader<&mut BufReader<&'b mut (dyn NetworkStream + 'static)>> {
        self.inner.body()
    }

    pub fn body_mut(&mut self) -> &mut HttpReader<&'a mut BufReader<&'b mut (dyn NetworkStream + 'static)>> {
        self.inner.body_mut()
    }

    /// Set the read timeout of the underlying NetworkStream.
    #[inline]
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.body().get_ref().get_ref().set_read_timeout(timeout)
    }

    /// Get a reference to the underlying `NetworkStream`.
    #[inline]
    pub fn downcast_ref<T: NetworkStream>(&self) -> Option<&T> {
        self.body().get_ref().get_ref().downcast_ref()
    }

    /// Get a reference to the underlying Ssl stream, if connected
    /// over HTTPS.
    ///
    /// This is actually just an alias for `downcast_ref`.
    #[inline]
    pub fn ssl<T: NetworkStream>(&self) -> Option<&T> {
        self.downcast_ref()
    }

    /// Deconstruct a Request into its constituent parts.
    #[inline]
    pub fn deconstruct(self) -> (SocketAddr, http::Method, http::HeaderMap,
                                 http::Uri, http::Version,
                                 HttpReader<&'a mut BufReader<&'b mut dyn NetworkStream>>) {
        let mut p = self.inner.into_parts();
        (p.0.extensions.get::<SocketAddr>().unwrap().clone(), p.0.method, p.0.headers,
         p.0.uri, p.0.version, p.1)
    }

    fn remote_addr(&self) -> &SocketAddr {
        self.inner.extensions().get::<SocketAddr>().unwrap()
    }

    fn set_remote_addr(&mut self, arg: SocketAddr) {
        // let mut ext = Extensions::new();
        // ext.insert(arg);
        self.inner.extensions_mut().insert(arg);
    }
}

impl<'a, 'b> Read for Request<'a, 'b> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.body_mut().read(buf)
    }
}

#[cfg(test)]
mod tests {
    use crate::buffer::BufReader;
    use crate::header::{Host, TransferEncoding, Encoding};
    use crate::net::NetworkStream;
    use crate::mock::MockStream;
    use super::Request;

    use std::io::{self, Read};
    use std::net::SocketAddr;

    fn sock(s: &str) -> SocketAddr {
        s.parse().unwrap()
    }

    fn read_to_string(mut req: Request) -> io::Result<String> {
        let mut s = String::new();
        r#try!(req.read_to_string(&mut s));
        Ok(s)
    }

    #[test]
    fn test_get_empty_body() {
        let mut mock = MockStream::with_input(b"\
            GET / HTTP/1.1\r\n\
            Host: example.domain\r\n\
            \r\n\
            I'm a bad request.\r\n\
        ");

        // FIXME: Use Type ascription
        let mock: &mut dyn NetworkStream = &mut mock;
        let mut stream = BufReader::new(mock);

        let req = Request::new(&mut stream, sock("127.0.0.1:80")).unwrap();
        assert_eq!(read_to_string(req).unwrap(), "".to_owned());
    }

    #[test]
    fn test_get_with_body() {
        let mut mock = MockStream::with_input(b"\
            GET / HTTP/1.1\r\n\
            Host: example.domain\r\n\
            Content-Length: 19\r\n\
            \r\n\
            I'm a good request.\r\n\
        ");

        // FIXME: Use Type ascription
        let mock: &mut dyn NetworkStream = &mut mock;
        let mut stream = BufReader::new(mock);

        let req = Request::new(&mut stream, sock("127.0.0.1:80")).unwrap();
        assert_eq!(read_to_string(req).unwrap(), "I'm a good request.".to_owned());
    }

    #[test]
    fn test_head_empty_body() {
        let mut mock = MockStream::with_input(b"\
            HEAD / HTTP/1.1\r\n\
            Host: example.domain\r\n\
            \r\n\
            I'm a bad request.\r\n\
        ");

        // FIXME: Use Type ascription
        let mock: &mut dyn NetworkStream = &mut mock;
        let mut stream = BufReader::new(mock);

        let req = Request::new(&mut stream, sock("127.0.0.1:80")).unwrap();
        assert_eq!(read_to_string(req).unwrap(), "".to_owned());
    }

    #[test]
    fn test_post_empty_body() {
        let mut mock = MockStream::with_input(b"\
            POST / HTTP/1.1\r\n\
            Host: example.domain\r\n\
            \r\n\
            I'm a bad request.\r\n\
        ");

        // FIXME: Use Type ascription
        let mock: &mut dyn NetworkStream = &mut mock;
        let mut stream = BufReader::new(mock);

        let req = Request::new(&mut stream, sock("127.0.0.1:80")).unwrap();
        assert_eq!(read_to_string(req).unwrap(), "".to_owned());
    }

    #[test]
    fn test_parse_chunked_request() {
        let mut mock = MockStream::with_input(b"\
            POST / HTTP/1.1\r\n\
            Host: example.domain\r\n\
            Transfer-Encoding: chunked\r\n\
            \r\n\
            1\r\n\
            q\r\n\
            2\r\n\
            we\r\n\
            2\r\n\
            rt\r\n\
            0\r\n\
            \r\n"
        );

        // FIXME: Use Type ascription
        let mock: &mut dyn NetworkStream = &mut mock;
        let mut stream = BufReader::new(mock);

        let req = Request::new(&mut stream, sock("127.0.0.1:80")).unwrap();

        // The headers are correct?
        match req.headers.get::<Host>() {
            Some(host) => {
                assert_eq!("example.domain", host.hostname);
            }
            None => panic!("Host header expected!"),
        };
        match req.headers.get::<TransferEncoding>() {
            Some(encodings) => {
                assert_eq!(1, encodings.len());
                assert_eq!(Encoding::Chunked, encodings[0]);
            }
            None => panic!("Transfer-Encoding: chunked expected!"),
        };
        // The content is correctly read?
        assert_eq!(read_to_string(req).unwrap(), "qwert".to_owned());
    }

    /// Tests that when a chunk size is not a valid radix-16 number, an error
    /// is returned.
    #[test]
    fn test_invalid_chunk_size_not_hex_digit() {
        let mut mock = MockStream::with_input(b"\
            POST / HTTP/1.1\r\n\
            Host: example.domain\r\n\
            Transfer-Encoding: chunked\r\n\
            \r\n\
            X\r\n\
            1\r\n\
            0\r\n\
            \r\n"
        );

        // FIXME: Use Type ascription
        let mock: &mut dyn NetworkStream = &mut mock;
        let mut stream = BufReader::new(mock);

        let req = Request::new(&mut stream, sock("127.0.0.1:80")).unwrap();

        assert!(read_to_string(req).is_err());
    }

    /// Tests that when a chunk size contains an invalid extension, an error is
    /// returned.
    #[test]
    fn test_invalid_chunk_size_extension() {
        let mut mock = MockStream::with_input(b"\
            POST / HTTP/1.1\r\n\
            Host: example.domain\r\n\
            Transfer-Encoding: chunked\r\n\
            \r\n\
            1 this is an invalid extension\r\n\
            1\r\n\
            0\r\n\
            \r\n"
        );

        // FIXME: Use Type ascription
        let mock: &mut dyn NetworkStream = &mut mock;
        let mut stream = BufReader::new(mock);

        let req = Request::new(&mut stream, sock("127.0.0.1:80")).unwrap();

        assert!(read_to_string(req).is_err());
    }

    /// Tests that when a valid extension that contains a digit is appended to
    /// the chunk size, the chunk is correctly read.
    #[test]
    fn test_chunk_size_with_extension() {
        let mut mock = MockStream::with_input(b"\
            POST / HTTP/1.1\r\n\
            Host: example.domain\r\n\
            Transfer-Encoding: chunked\r\n\
            \r\n\
            1;this is an extension with a digit 1\r\n\
            1\r\n\
            0\r\n\
            \r\n"
        );

        // FIXME: Use Type ascription
        let mock: &mut dyn NetworkStream = &mut mock;
        let mut stream = BufReader::new(mock);

        let req = Request::new(&mut stream, sock("127.0.0.1:80")).unwrap();

        assert_eq!(read_to_string(req).unwrap(), "1".to_owned());
    }
}
