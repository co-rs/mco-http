use crate::header::{Header, Host, HeaderFormat};
use std::fmt;
use std::str::FromStr;
use crate::header::parsing::from_one_raw_str;

/// The `Origin` header.
///
/// The `Origin` header is a version of the `Referer` header that is used for all HTTP fetches and `POST`s whose CORS flag is set.
/// This header is often used to inform recipients of the security context of where the request was initiated.
///
///
/// Following the spec, https://fetch.spec.whatwg.org/#origin-header, the value of this header is composed of
/// a String (scheme), header::Host (host/port)
///
/// # Examples
/// ```
/// use mco_http::header::{Headers, Origin};
///
/// let mut headers = Headers::new();
/// headers.set(
///     Origin::new("http", "mco_http.rs", None)
/// );
/// ```
/// ```
/// use mco_http::header::{Headers, Origin};
///
/// let mut headers = Headers::new();
/// headers.set(
///     Origin::new("https", "wikipedia.org", Some(443))
/// );
/// ```

#[derive(Clone, Debug)]
pub struct Origin {
    /// The scheme, such as http or https
    pub scheme: String,
    /// The host, such as Host{hostname: "mco_http.rs".to_owned(), port: None}
    pub host: Host,
}

impl Origin {
    /// Creates a new `Origin` header.
    pub fn new<S: Into<String>, H: Into<String>>(scheme: S, hostname: H, port: Option<u16>) -> Origin{
        Origin {
            scheme: scheme.into(),
            host: Host {
                hostname: hostname.into(),
                port: port
            }
        }
    }
}

impl Header for Origin {
    fn header_name() -> &'static str {
        static NAME: &'static str = "Origin";
        NAME
    }

    fn parse_header(raw: &[Vec<u8>]) -> crate::Result<Origin> {
        from_one_raw_str(raw)
    }
}

impl FromStr for Origin {
    type Err = crate::Error;

    fn from_str(s: &str) -> crate::Result<Origin> {
        let idx = match s.find("://") {
            Some(idx) => idx,
            None => return Err(crate::Error::Header)
        };
        // idx + 3 because thats how long "://" is
        let (scheme, etc) = (&s[..idx], &s[idx + 3..]);
        let host = Host::from_str(etc)?;


        Ok(Origin{
            scheme: scheme.to_owned(),
            host: host
        })
    }
}

impl HeaderFormat for Origin {
    fn fmt_header(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}://{}", self.scheme, self.host)
    }
}

impl PartialEq for Origin {
    fn eq(&self, other: &Origin) -> bool {
        self.scheme == other.scheme && self.host == other.host
    }
}


#[cfg(test)]
mod tests {
    use super::Origin;
    use crate::header::Header;

    #[test]
    fn test_origin() {
        let origin = Header::parse_header([b"http://foo.com".to_vec()].as_ref());
        assert_eq!(origin.ok(), Some(Origin::new("http", "foo.com", None)));

        let origin = Header::parse_header([b"https://foo.com:443".to_vec()].as_ref());
        assert_eq!(origin.ok(), Some(Origin::new("https", "foo.com", Some(443))));
    }
}

bench_header!(bench, Origin, { vec![b"https://foo.com".to_vec()] });
