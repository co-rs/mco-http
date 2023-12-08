extern crate mco_http;
extern crate rustls;
extern crate webpki_roots;

use mco_http::net::{HttpStream, NetworkStream};
use std::convert::{TryInto};

use std::{io};
use std::fmt::{Debug, Display, Formatter};
use std::io::{BufReader, Cursor, Read, Write};
use std::net::{Shutdown, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use rustls::{ClientConfig, ClientConnection, ConfigBuilder, RootCertStore, ServerConnection, StreamOwned, WantsVerifier};
use rustls::client::WantsClientCert;
use mco_http::runtime::Mutex;


pub enum Connection {
    Client(StreamOwned<ClientConnection, HttpStream>),
    Server(StreamOwned<ServerConnection, HttpStream>),
}

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Connection::Client(c) => {
                c.read(buf)
            }
            Connection::Server(c) => {
                c.read(buf)
            }
        }
    }
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Connection::Client(c) => {
                c.write(buf)
            }
            Connection::Server(c) => {
                c.write(buf)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Connection::Client(c) => {
                c.flush()
            }
            Connection::Server(c) => {
                c.flush()
            }
        }
    }
}


pub struct TlsStream {
    sess: Box<Connection>,
    tls_error: Option<rustls::Error>,
}

impl TlsStream {
    fn promote_tls_error(&mut self) -> io::Result<()> {
        match self.tls_error.take() {
            Some(err) => {
                return Err(io::Error::new(io::ErrorKind::ConnectionAborted, err));
            }
            None => return Ok(()),
        };
    }
}

impl NetworkStream for TlsStream {
    fn peer_addr(&mut self) -> io::Result<SocketAddr> {
        match self.sess.as_mut() {
            Connection::Client(c) => {
                c.sock.peer_addr()
            }
            Connection::Server(c) => {
                c.sock.peer_addr()
            }
        }
    }

    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        match self.sess.as_ref() {
            Connection::Client(c) => {
                c.sock.set_read_timeout(dur)
            }
            Connection::Server(c) => {
                c.sock.set_read_timeout(dur)
            }
        }
    }

    fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        match self.sess.as_ref() {
            Connection::Client(c) => {
                c.sock.set_write_timeout(dur)
            }
            Connection::Server(c) => {
                c.sock.set_write_timeout(dur)
            }
        }
    }
}

impl io::Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            self.promote_tls_error()?;
            match self.sess.as_mut().read(buf) {
                Ok(0) => continue,
                Ok(n) => return Ok(n),
                Err(e) => return Err(e),
            }
        }
    }
}

impl io::Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.sess.write(buf)?;
        self.promote_tls_error()?;
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        let rc = self.sess.flush();
        self.promote_tls_error()?;
        rc
    }
}

#[derive(Clone)]
pub struct WrappedStream(Arc<Mutex<TlsStream>>);

impl WrappedStream {
    fn lock(&self) -> mco_http::runtime::MutexGuard<TlsStream> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl io::Read for WrappedStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.lock().read(buf)
    }
}

impl io::Write for WrappedStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.lock().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.lock().flush()
    }
}

impl NetworkStream for WrappedStream {
    fn peer_addr(&mut self) -> io::Result<SocketAddr> {
        self.lock().peer_addr()
    }

    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.lock().set_read_timeout(dur)
    }

    fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.lock().set_write_timeout(dur)
    }

    fn close(&mut self, how: Shutdown) -> io::Result<()> {
        self.lock().close(how)
    }
}

pub struct TlsClient {
    pub cfg: Arc<rustls::ClientConfig>,
}

impl TlsClient {
    pub fn new() -> TlsClient {
        // let mut root_store = RootCertStore::empty();
        // root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        // let mut config = rustls::ClientConfig::builder()
        //     .with_root_certificates(root_store)
        //     .with_no_client_auth();
        //
        // // Allow using SSLKEYLOGFILE.
        // config.key_log = Arc::new(rustls::KeyLogFile::new());
        //
        // TlsClient {
        //     cfg: Arc::new(config),
        // }
        return Self::new_ca(None).expect("crate TlsClient fail")
    }

    pub fn new_ca(mut ca:Option<&mut dyn io::BufRead>)-> Result<TlsClient,io::Error> {
        // Prepare the TLS client config
        let tls = match ca {
            Some(ref mut rd) => {
                // Read trust roots
                let certs = rustls_pemfile::certs(rd).collect::<Result<Vec<_>, _>>()?;
                let mut roots = RootCertStore::empty();
                roots.add_parsable_certificates(certs);
                // TLS client config using the custom CA store for lookups
                rustls::ClientConfig::builder()
                    .with_root_certificates(roots)
                    .with_no_client_auth()
            }
            // Default TLS client config with native roots
            None => rustls::ClientConfig::builder()
                .with_native_roots()?
                .with_no_client_auth(),
        };
        Ok(Self{
            cfg: Arc::new(tls),
        })
    }
}


/// Methods for configuring roots
///
/// This adds methods (gated by crate features) for easily configuring
/// TLS server roots a rustls ClientConfig will trust.
pub trait ConfigBuilderExt {
    /// This configures the platform's trusted certs, as implemented by
    /// rustls-native-certs
    ///
    /// This will return an error if no valid certs were found. In that case,
    /// it's recommended to use `with_webpki_roots`.
    //#[cfg(feature = "rustls-native-certs")]
    fn with_native_roots(self) -> std::io::Result<ConfigBuilder<ClientConfig, WantsClientCert>>;

    /// This configures the webpki roots, which are Mozilla's set of
    /// trusted roots as packaged by webpki-roots.
    //#[cfg(feature = "webpki-roots")]
    fn with_webpki_roots(self) -> ConfigBuilder<ClientConfig, WantsClientCert>;
}

impl ConfigBuilderExt for ConfigBuilder<ClientConfig, WantsVerifier> {
    //#[cfg(feature = "rustls-native-certs")]
    //#[cfg_attr(not(feature = "logging"), allow(unused_variables))]
    fn with_native_roots(self) -> std::io::Result<ConfigBuilder<ClientConfig, WantsClientCert>> {
        let mut roots = rustls::RootCertStore::empty();
        let mut valid_count = 0;
        let mut invalid_count = 0;

        for cert in rustls_native_certs::load_native_certs().expect("could not load platform certs")
        {
            match roots.add(cert) {
                Ok(_) => valid_count += 1,
                Err(err) => {
                    log::debug!("certificate parsing failed: {:?}", err);
                    invalid_count += 1
                }
            }
        }
        log::debug!(
            "with_native_roots processed {} valid and {} invalid certs",
            valid_count,
            invalid_count
        );
        if roots.is_empty() {
            log::debug!("no valid native root CA certificates found");
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no valid native root CA certificates found ({invalid_count} invalid)"),
            ))?
        }

        Ok(self.with_root_certificates(roots))
    }

    //#[cfg(feature = "webpki-roots")]
    fn with_webpki_roots(self) -> ConfigBuilder<ClientConfig, WantsClientCert> {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(
            webpki_roots::TLS_SERVER_ROOTS
                .iter()
                .cloned(),
        );
        self.with_root_certificates(roots)
    }
}





#[derive(Debug)]
pub struct DNSError {
    pub inner: String,
}

impl Display for DNSError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

impl std::error::Error for DNSError {}

impl mco_http::net::SslClient for TlsClient {
    type Stream = WrappedStream;

    fn wrap_client(&self, stream: HttpStream, host: &str) -> mco_http::Result<WrappedStream> {
        let c = ClientConnection::new(
            self.cfg.clone(),
            host.to_string().try_into().unwrap(),
        ).map_err(|e| mco_http::Error::Ssl(Box::new(e)))?;
        let tls = TlsStream {
            sess: Box::new(Connection::Client(StreamOwned::new(c, stream))),
            tls_error: None,
        };

        Ok(WrappedStream(Arc::new(Mutex::new(tls))))
    }
}


pub use SSLServer as TlsServer;

#[derive(Clone)]
pub struct SSLServer {
    pub cfg: Arc<rustls::ServerConfig>,
}

impl SSLServer {
    pub fn new(certs: Vec<Vec<u8>>, key: Vec<u8>) -> SSLServer {
        let flattened_data: Vec<u8> = certs.into_iter().flatten().collect();
        let mut reader = BufReader::new(Cursor::new(flattened_data));
        let certs = rustls_pemfile::certs(&mut reader).map(|result| result.unwrap())
            .collect();
        let private_key=rustls_pemfile::private_key(&mut BufReader::new(Cursor::new(key.clone()))).expect("rustls_pemfile::private_key() read fail");
        if private_key.is_none() {
            panic!("load keys is empty")
        }
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, private_key.unwrap()).unwrap();

        SSLServer {
            cfg: Arc::new(config),
        }
    }
}

impl mco_http::net::SslServer for SSLServer {
    type Stream = WrappedStream;

    fn wrap_server(&self, stream: HttpStream) -> mco_http::Result<WrappedStream> {
        let conn = ServerConnection::new(self.cfg.clone()).unwrap();
        let stream = rustls::StreamOwned::new(conn, stream);

        let tls = TlsStream {
            sess: Box::new(Connection::Server(stream)),
            tls_error: None,
        };

        Ok(WrappedStream(Arc::new(Mutex::new(tls))))
    }
}
