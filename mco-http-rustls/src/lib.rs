extern crate mco_http;
extern crate rustls;
extern crate vecio;
extern crate webpki_roots;

use mco_http::net::{HttpStream, NetworkStream};
use std::convert::TryInto;

use std::io;
use std::net::{Shutdown, SocketAddr};
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use rustls::{ClientConnection, RootCertStore};

pub struct TlsStream {
    sess: Box<ClientConnection>,
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
            try!(self.promote_tls_error());
            match self.sess.read(buf) {
                Ok(0) => continue,
                Ok(n) => return Ok(n),
                Err(e) => return Err(e),
            }
        }
    }
}

impl io::Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = try!(self.sess.write(buf));
        try!(self.promote_tls_error());
        self.underlying_io();
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        let rc = self.sess.flush();
        try!(self.promote_tls_error());
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

impl mco_http::net::SslClient for TlsClient {
    type Stream = WrappedStream;

    fn wrap_client(&self, stream: HttpStream, host: &str) -> mco_http::Result<WrappedStream> {
        let tls = TlsStream {
            sess: Box::new(rustls::ClientConnection::new(
                self.cfg.clone(),
                host.try_into().unwrap(),
            )?),
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
        let mut config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth();

        let certs = rustls_pemfile::certs(&mut certs.as_slice()).unwrap();
        let key = rustls_pemfile::rsa_private_keys(&mut key.as_slice()).unwrap();

        config.with_single_cert(certs, key[0].clone()).unwrap();

        TlsServer {
            cfg: Arc::new(config),
        }
    }
}

impl mco_http::net::SslServer for TlsServer {
    type Stream = WrappedStream;

    fn wrap_server(&self, stream: HttpStream) -> mco_http::Result<WrappedStream> {
        let tls = TlsStream {
            sess: Box::new(
                rustls::ServerConnection::new(self.cfg.clone())
                    .map_err(|e| mco_http::Error::from(e.to_string()))?,
            )
            .map_err(|e| mco_http::Error::from(e.to_string()))?,
            underlying: stream,
            io_error: None,
            tls_error: None,
        };

        Ok(WrappedStream(Arc::new(Mutex::new(tls))))
    }
}
