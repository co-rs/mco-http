extern crate mco_http;
extern crate rustls;
extern crate webpki_roots;

use mco_http::net::{HttpsStream, HttpStream, NetworkStream};
use std::convert::{TryInto};

use std::{io};
use std::fmt::{Debug, Display, Formatter};
use std::io::{BufReader, Cursor, Error, Read, Write};
use std::net::{Shutdown, SocketAddr};
use std::ops::DerefMut;
use std::sync::Arc;
use std::time::Duration;

use rustls::{ClientConnection, IoState, Reader, RootCertStore, ServerConnection, StreamOwned, Writer};
use rustls::server::Acceptor;
use mco_http::runtime::Mutex;


pub enum Connection{
    Client(StreamOwned<ClientConnection,HttpStream>),
    Server(StreamOwned<ServerConnection,HttpStream>)
}

impl Read for Connection{
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

impl Write for Connection{
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
    io_error: Option<io::Error>,
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

impl NetworkStream for TlsStream{
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
        let c=ClientConnection::new(
            self.cfg.clone(),
            host.to_string().try_into().unwrap(),
        ).map_err(|e|mco_http::Error::Ssl(Box::new(e)))?;
        let tls = TlsStream {
            sess: Box::new(Connection::Client(StreamOwned::new(c,stream))),
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

    fn wrap_server(&self, mut stream: HttpStream) -> mco_http::Result<WrappedStream> {
        // let mut acceptor = Acceptor::default();
        // let accepted = loop {
        //     acceptor.read_tls(&mut stream).unwrap();
        //     if let Some(accepted) = acceptor.accept().unwrap() {
        //         break accepted;
        //     }
        // };
        // let conn = accepted
        //     .into_connection(self.cfg.clone())
        //     .unwrap();
        let  conn = ServerConnection::new(self.cfg.clone()).unwrap();
        let  stream = rustls::StreamOwned::new(conn, stream);

        let tls = TlsStream {
            sess: Box::new(Connection::Server(stream)),
            io_error: None,
            tls_error: None,
        };

        Ok(WrappedStream(Arc::new(Mutex::new(tls))))
    }
}
