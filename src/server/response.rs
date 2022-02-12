//! Server Responses
//!
//! These are responses sent by a `cogo_http::Server` to clients, after
//! receiving a request.
use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::mem;
use std::io::{self, Write};
use std::ptr;
use std::thread;
use http::HeaderValue;

use time::now_utc;
use crate::proto::h1::{LINE_ENDING, HttpWriter};
use crate::proto::h1::HttpWriter::{ThroughWriter, ChunkedWriter, SizedWriter, EmptyWriter};
use crate::{header_value, status};
use crate::net::{Fresh, Streaming};
use crate::version;


pub type ResponseNew<'a> = http::Response<HttpWriter<&'a mut (dyn Write + 'a)>>;


/// The outgoing half for a Tcp connection, created by a `Server` and given to a `Handler`.
///
/// The default `StatusCode` for a `Response` is `200 OK`.
///
/// There is a `Drop` implementation for `Response` that will automatically
/// write the head and flush the body, if the handler has not already done so,
/// so that the server doesn't accidentally leave dangling requests.
#[derive(Debug)]
pub struct Response<'a, W: Any = Fresh> {
    /// The HTTP version of this response.
    pub version: http::Version,
    // Stream the Response is writing to, not accessible through UnwrittenResponse
    pub body: HttpWriter<&'a mut (dyn Write + 'a)>,
    // The status code for the request.
    pub status: http::StatusCode,
    // The outgoing headers on this response.
    pub headers: &'a mut http::HeaderMap,

    _writing: PhantomData<W>,
}

impl<'a, W: Any> Response<'a, W> {
    /// The status of this response.
    #[inline]
    pub fn status(&self) -> http::StatusCode { self.status }

    /// The headers of this response.
    #[inline]
    pub fn headers(&self) -> &http::HeaderMap { &*self.headers }

    /// Construct a Response from its constituent parts.
    #[inline]
    pub fn construct(version: http::Version,
                     body: HttpWriter<&'a mut (dyn Write + 'a)>,
                     status: http::StatusCode,
                     headers: &'a mut http::HeaderMap) -> Response<'a, Fresh> {
        Response {
            status: status,
            version: version,
            body: body,
            headers: headers,
            _writing: PhantomData,
        }
    }

    /// Deconstruct this Response into its constituent parts.
    #[inline]
    pub fn deconstruct(self) -> (http::Version, HttpWriter<&'a mut (dyn Write + 'a)>,
                                 http::StatusCode, &'a mut http::HeaderMap) {
        unsafe {
            let parts = (
                self.version,
                ptr::read(&self.body),
                self.status,
                ptr::read(&self.headers)
            );
            mem::forget(self);
            parts
        }
    }

    fn write_head(&mut self) -> io::Result<Body> {
        debug!("writing head: {:?} {:?}", self.version, self.status);
        r#try!(write!(&mut self.body, "{:?} {}\r\n", self.version, self.status));

        if !self.headers.contains_key("Date") {
            let date = httpdate::HttpDate::from(std::time::SystemTime::now()).to_string();
            self.headers.insert(http::header::DATE, http::HeaderValue::from_bytes(date.as_bytes()).unwrap());
        }

        let body_type = match self.status {
            http::StatusCode::NO_CONTENT | http::StatusCode::NOT_MODIFIED => Body::Empty,
            c if c.as_u16() < 200 => Body::Empty,
            _ => if let Some(cl) = self.headers.get(http::header::CONTENT_LENGTH) {
                Body::Sized(cl.to_str().unwrap_or_default().parse().unwrap_or_default())
            } else {
                Body::Chunked
            }
        };

        // can't do in match above, thanks borrowck
        if body_type == Body::Chunked {
            let encodings = match self.headers.get_mut("Transfer-Encoding") {
                Some(encodings) => {
                    //check if chunked is already in encodings. use HashSet?
                    let estr= encodings.to_str().unwrap_or_default();
                    if !estr.contains("chunked"){
                        *encodings = header_value!(&(estr.to_string()+";chunked"));
                    }
                    false
                }
                None => true
            };

            if encodings {
                self.headers.insert(http::header::TRANSFER_ENCODING , HeaderValue::from_static("chunked"));
            }
        }

        debug!("headers [\n{:?}]", self.headers);

        for (name, value) in self.headers.iter() {
            self.body.write(name.as_str().as_bytes())?;
            self.body.write(":".as_bytes())?;
            self.body.write(value.as_bytes())?;
            self.body.write("\r\n".as_bytes())?;
        }

        r#try!(write!(&mut self.body, "{}", LINE_ENDING));

        Ok(body_type)
    }
}

impl<'a> Response<'a, Fresh> {
    /// Creates a new Response that can be used to write to a network stream.
    #[inline]
    pub fn new(stream: &'a mut (dyn Write + 'a), headers: &'a mut http::HeaderMap) ->
    Response<'a, Fresh> {
        Response {
            status: http::StatusCode::OK,
            version: http::Version::HTTP_11,
            headers: headers,
            body: ThroughWriter(stream),
            _writing: PhantomData,
        }
    }

    /// Writes the body and ends the response.
    ///
    /// This is a shortcut method for when you have a response with a fixed
    /// size, and would only need a single `write` call normally.
    ///
    /// # Example
    ///
    /// ```
    /// # use cogo_http::server::Response;
    /// fn handler(res: Response) {
    ///     res.send(b"Hello World!").unwrap();
    /// }
    /// ```
    ///
    /// The above is the same, but shorter, than the longer:
    ///
    /// ```
    /// # use cogo_http::server::Response;
    /// use std::io::Write;
    /// use cogo_http::header::ContentLength;
    /// fn handler(mut res: Response) {
    ///     let body = b"Hello World!";
    ///     res.headers_mut().set(ContentLength(body.len() as u64));
    ///     let mut res = res.start().unwrap();
    ///     res.write_all(body).unwrap();
    /// }
    /// ```
    #[inline]
    pub fn send(self, body: &[u8]) -> io::Result<()> {
        self.headers.insert(http::header::CONTENT_LENGTH, http::HeaderValue::from_str(&body.len().to_string()).unwrap());
        let mut stream = r#try!(self.start());
        r#try!(stream.write_all(body));
        stream.end()
    }

    /// Consume this Response<Fresh>, writing the Headers and Status and
    /// creating a Response<Streaming>
    pub fn start(mut self) -> io::Result<Response<'a, Streaming>> {
        let body_type = r#try!(self.write_head());
        let (version, body, status, headers) = self.deconstruct();
        let stream = match body_type {
            Body::Chunked => ChunkedWriter(body.into_inner()),
            Body::Sized(len) => SizedWriter(body.into_inner(), len),
            Body::Empty => EmptyWriter(body.into_inner()),
        };
        // "copy" to change the phantom type
        Ok(Response {
            version: version,
            body: stream,
            status: status,
            headers: headers,
            _writing: PhantomData,
        })
    }
    /// Get a mutable reference to the status.
    #[inline]
    pub fn status_mut(&mut self) -> &mut http::StatusCode { &mut self.status }

    /// Get a mutable reference to the Headers.
    #[inline]
    pub fn headers_mut(&mut self) -> &mut http::HeaderMap { self.headers }
}


impl<'a> Response<'a, Streaming> {
    /// Flushes all writing of a response to the client.
    #[inline]
    pub fn end(self) -> io::Result<()> {
        trace!("ending");
        let (_, body, _, _) = self.deconstruct();
        r#try!(body.end());
        Ok(())
    }
}

impl<'a> Write for Response<'a, Streaming> {
    #[inline]
    fn write(&mut self, msg: &[u8]) -> io::Result<usize> {
        debug!("write {:?} bytes", msg.len());
        self.body.write(msg)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.body.flush()
    }
}

#[derive(PartialEq)]
enum Body {
    Chunked,
    Sized(u64),
    Empty,
}

impl<'a, T: Any> Drop for Response<'a, T> {
    fn drop(&mut self) {
        if TypeId::of::<T>() == TypeId::of::<Fresh>() {
            if thread::panicking() {
                self.status = http::StatusCode::INTERNAL_SERVER_ERROR;
            }
            let mut body = match self.write_head() {
                Ok(Body::Chunked) => ChunkedWriter(self.body.get_mut()),
                Ok(Body::Sized(len)) => SizedWriter(self.body.get_mut(), len),
                Ok(Body::Empty) => EmptyWriter(self.body.get_mut()),
                Err(e) => {
                    debug!("error dropping request: {:?}", e);
                    return;
                }
            };
            end(&mut body);
        } else {
            end(&mut self.body);
        };


        #[inline]
        fn end<W: Write>(w: &mut W) {
            match w.write(&[]) {
                Ok(_) => match w.flush() {
                    Ok(_) => debug!("drop successful"),
                    Err(e) => debug!("error dropping request: {:?}", e)
                },
                Err(e) => debug!("error dropping request: {:?}", e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mock::MockStream;
    use crate::runtime;
    use crate::status::StatusCode;
    use super::Response;

    macro_rules! lines {
        ($s:ident = $($line:pat),+) => ({
            let s = String::from_utf8($s.write).unwrap();
            let mut lines = s.split_terminator("\r\n");

            $(
                match lines.next() {
                    Some($line) => (),
                    other => panic!("line mismatch: {:?} != {:?}", other, stringify!($line))
                }
            )+

            assert_eq!(lines.next(), None);
        })
    }

    #[test]
    fn test_fresh_start() {
        let mut headers = Headers::new();
        let mut stream = MockStream::new();
        {
            let res = Response::new(&mut stream, &mut headers);
            res.start().unwrap().deconstruct();
        }

        lines! { stream =
            "HTTP/1.1 200 OK",
            _date,
            _transfer_encoding,
            ""
        }
    }

    #[test]
    fn test_streaming_end() {
        let mut headers = Headers::new();
        let mut stream = MockStream::new();
        {
            let res = Response::new(&mut stream, &mut headers);
            res.start().unwrap().end().unwrap();
        }

        lines! { stream =
            "HTTP/1.1 200 OK",
            _date,
            _transfer_encoding,
            "",
            "0",
            "" // empty zero body
        }
    }

    #[test]
    fn test_fresh_drop() {
        use crate::status::StatusCode;
        let mut headers = Headers::new();
        let mut stream = MockStream::new();
        {
            let mut res = Response::new(&mut stream, &mut headers);
            *res.status_mut() = StatusCode::NotFound;
        }

        lines! { stream =
            "HTTP/1.1 404 Not Found",
            _date,
            _transfer_encoding,
            "",
            "0",
            "" // empty zero body
        }
    }

    // x86 windows msvc does not support unwinding
    // See https://github.com/rust-lang/rust/issues/25869
    #[cfg(not(all(windows, target_arch = "x86", target_env = "msvc")))]
    #[test]
    fn test_fresh_drop_panicing() {
        use std::thread;
        use std::sync::{Arc, Mutex};

        use crate::status::StatusCode;

        let stream = MockStream::new();
        let stream = Arc::new(Mutex::new(stream));
        let inner_stream = stream.clone();
        let join_handle = runtime::spawn(move || {
            let mut headers = Headers::new();
            let mut stream = inner_stream.lock().unwrap();
            let mut res = Response::new(&mut *stream, &mut headers);
            *res.status_mut() = StatusCode::NotFound;

            panic!("inside")
        });

        assert!(join_handle.join().is_err());

        let stream = match stream.lock() {
            Err(poisoned) => poisoned.into_inner().clone(),
            Ok(_) => unreachable!()
        };

        lines! { stream =
            "HTTP/1.1 500 Internal Server Error",
            _date,
            _transfer_encoding,
            "",
            "0",
            "" // empty zero body
        }
    }


    #[test]
    fn test_streaming_drop() {
        use std::io::Write;
        use crate::status::StatusCode;
        let mut headers = Headers::new();
        let mut stream = MockStream::new();
        {
            let mut res = Response::new(&mut stream, &mut headers);
            *res.status_mut() = StatusCode::NotFound;
            let mut stream = res.start().unwrap();
            stream.write_all(b"foo").unwrap();
        }

        lines! { stream =
            "HTTP/1.1 404 Not Found",
            _date,
            _transfer_encoding,
            "",
            "3",
            "foo",
            "0",
            "" // empty zero body
        }
    }

    #[test]
    fn test_no_content() {
        let mut headers = Headers::new();
        let mut stream = MockStream::new();
        {
            let mut res = Response::new(&mut stream, &mut headers);
            *res.status_mut() = StatusCode::NoContent;
            res.start().unwrap();
        }

        lines! { stream =
            "HTTP/1.1 204 No Content",
            _date,
            ""
        }
    }
}
