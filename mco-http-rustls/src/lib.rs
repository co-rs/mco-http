extern crate mco_http;
extern crate rustls;
extern crate vecio;
extern crate webpki_roots;
extern crate rustls_pki_types;

use mco_http::net::{HttpStream, NetworkStream};
use std::convert::{TryInto};

use std::{io};
use std::fmt::{Debug, Display, Formatter};
use std::io::{BufReader, Cursor, Error};
use std::net::{Shutdown, SocketAddr};
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use rustls::{ClientConnection, IoState, Reader, RootCertStore, ServerConnection, Writer};


pub enum Connection{
    Client(ClientConnection),
    Server(ServerConnection)
}

impl Connection{
    pub fn writer(&mut self) -> Writer<'_> {
        match self {
            Connection::Client(c) => {c.writer()}
            Connection::Server(c) => {c.writer()}
        }
    }
    pub fn reader(&mut self) -> Reader<'_> {
        match self {
            Connection::Client(c) => {c.reader()}
            Connection::Server(c) => {c.reader()}
        }
    }
    pub fn process_new_packets(&mut self) -> Result<IoState, rustls::Error> {
        match self {
            Connection::Client(c) => {c.process_new_packets()}
            Connection::Server(c) => {c.process_new_packets()}
        }
    }
    pub fn wants_write(&self) -> bool {
        match self {
            Connection::Client(c) => {c.wants_write()}
            Connection::Server(c) => {c.wants_write()}
        }
    }

    pub fn wants_read(&self) -> bool {
        match self {
            Connection::Client(c) => {c.wants_read()}
            Connection::Server(c) => {c.wants_read()}
        }
    }
    pub fn write_tls(&mut self, wr: &mut dyn io::Write) -> Result<usize, Error> {
        match self {
            Connection::Client(c) => {c.write_tls(wr)}
            Connection::Server(c) => {c.write_tls(wr)}
        }
    }

    pub fn read_tls(&mut self, wr: &mut dyn io::Read) -> Result<usize, Error> {
        match self {
            Connection::Client(c) => {c.read_tls(wr)}
            Connection::Server(c) => {c.read_tls(wr)}
        }
    }
}


pub struct TlsStream {
    sess: Box<Connection>,
    underlying: HttpStream,
    tls_error: Option<rustls::Error>,
    io_error: Option<io::Error>,
}

impl TlsStream {
    fn underlying_io(&mut self) {
        if self.io_error.is_some() || self.tls_error.is_some() {
            return;
        }

        while self.io_error.is_none() && self.sess.wants_write() {
            if let Err(err) = self.sess.write_tls(&mut self.underlying) {
                if err.kind() != io::ErrorKind::WouldBlock {
                    self.io_error = Some(err);
                }
            }
        }

        if self.io_error.is_none() && self.sess.wants_read() {
            if let Err(err) = self.sess.read_tls(&mut self.underlying) {
                if err.kind() != io::ErrorKind::WouldBlock {
                    self.io_error = Some(err);
                }
            }
        }

        if let Err(err) = self.sess.process_new_packets() {
            self.tls_error = Some(err);
        }
    }

    fn promote_tls_error(&mut self) -> io::Result<()> {
        match self.tls_error.take() {
            Some(err) => {
                return Err(io::Error::new(io::ErrorKind::ConnectionAborted, err));
            }
            None => return Ok(()),
        };
    }

    fn close(&mut self, how: Shutdown) -> io::Result<()> {
        self.underlying.close(how)
    }

    fn peer_addr(&mut self) -> io::Result<SocketAddr> {
        self.underlying.peer_addr()
    }

    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.underlying.set_read_timeout(dur)
    }

    fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.underlying.set_write_timeout(dur)
    }
}

impl io::Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            self.underlying_io();
            self.promote_tls_error()?;
            match self.sess.as_mut().reader().read(buf) {
                Ok(0) => continue,
                Ok(n) => return Ok(n),
                Err(e) => return Err(e),
            }
        }
    }
}

impl io::Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.sess.writer().write(buf)?;
        self.promote_tls_error()?;
        self.underlying_io();
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        let rc = self.sess.writer().flush();
        self.promote_tls_error()?;
        self.underlying_io();
        rc
    }
}

#[derive(Clone)]
pub struct WrappedStream(Arc<Mutex<TlsStream>>);

impl WrappedStream {
    fn lock(&self) -> MutexGuard<TlsStream> {
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
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let mut config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        // Allow using SSLKEYLOGFILE.
        config.key_log = Arc::new(rustls::KeyLogFile::new());

        TlsClient {
            cfg: Arc::new(config),
        }
    }
}


#[derive(Debug)]
pub struct DNSError{
  pub inner:String
}

impl Display for DNSError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

impl std::error::Error for DNSError{

}

impl mco_http::net::SslClient for TlsClient {
    type Stream = WrappedStream;

    fn wrap_client(&self, stream: HttpStream, host: &str) -> mco_http::Result<WrappedStream> {
        let tls = TlsStream {
            sess: Box::new(Connection::Client(ClientConnection::new(
                self.cfg.clone(),
                host.to_string().try_into().unwrap(),
            ).map_err(|e|mco_http::Error::Ssl(Box::new(e)))?)),
            underlying: stream,
            io_error: None,
            tls_error: None,
        };

        Ok(WrappedStream(Arc::new(Mutex::new(tls))))
    }
}

#[derive(Clone)]
pub struct TlsServer {
    pub cfg: Arc<rustls::ServerConfig>,
}

impl TlsServer {
    pub fn new(certs: Vec<Vec<u8>>, key: Vec<u8>) -> TlsServer {
        let flattened_data: Vec<u8> = certs.into_iter().flatten().collect();
        let mut reader = BufReader::new(Cursor::new(flattened_data));
        let mut keys = BufReader::new(Cursor::new(key));
        let certs = rustls_pemfile::certs(&mut reader).map(|result| result.unwrap())
            .collect();
        let mut keys = rustls_pemfile::pkcs8_private_keys(&mut keys)
            .map(|result| result.unwrap())
            .collect::<Vec<_>>();
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, keys.pop().unwrap().into()).unwrap();

        TlsServer {
            cfg: Arc::new(config),
        }
    }
}

impl mco_http::net::SslServer for TlsServer {
    type Stream = WrappedStream;

    fn wrap_server(&self, stream: HttpStream) -> mco_http::Result<WrappedStream> {
        let v=
            rustls::ServerConnection::new(self.cfg.clone())
                .map_err(|e| mco_http::Error::Ssl(Box::new(e)))?;

        let tls = TlsStream {
            sess: Box::new(Connection::Server(v)),
            underlying: stream,
            io_error: None,
            tls_error: None,
        };

        Ok(WrappedStream(Arc::new(Mutex::new(tls))))
    }
}
