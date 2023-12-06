#[deny(unused_variables)]
extern crate mco_http;
extern crate fast_log;

use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use fast_log::config::Config;
use rustls::{Certificate, PrivateKey, ServerConfig, ServerConnection};
use mco_http::net::{HttpStream, NetworkStream, SslServer};
use mco_http::server::{Request, Response};

#[derive(Debug)]
pub struct MyHttpsStream {
    pub cfg: Arc<ServerConfig>,
    pub inner: rustls::StreamOwned<ServerConnection, HttpStream>,
}

impl Clone for MyHttpsStream {
    fn clone(&self) -> Self {
        let conn = ServerConnection::new(self.cfg.clone()).unwrap();
        Self {
            cfg: self.cfg.clone(),
            inner: rustls::StreamOwned::new(conn, self.inner.sock.clone()),
        }
    }
}

impl Read for MyHttpsStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Write for MyHttpsStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl NetworkStream for MyHttpsStream {
    fn peer_addr(&mut self) -> std::io::Result<SocketAddr> {
        self.inner.sock.peer_addr().map_err(|e| {
            println!("io error={}",e);
            e
        })
    }

    fn set_read_timeout(&self, dur: Option<Duration>) -> std::io::Result<()> {
        self.inner.sock.set_read_timeout(dur).map_err(|e| {
            println!("io error={}",e);
            e
        })
    }

    fn set_write_timeout(&self, dur: Option<Duration>) -> std::io::Result<()> {
        self.inner.sock.set_write_timeout(dur).map_err(|e| {
            println!("io error={}",e);
            e
        })
    }
}

#[derive(Clone, Debug)]
pub struct RustlsSSL {
    pub cfg: Arc<ServerConfig>,
}

impl SslServer for RustlsSSL {
    type Stream = MyHttpsStream;

    fn wrap_server(&self, stream: HttpStream) -> mco_http::Result<Self::Stream> {
        let conn = ServerConnection::new(self.cfg.clone()).unwrap();
        let mut stream = rustls::StreamOwned::new(conn, stream);
        println!("created stream");
        Ok(MyHttpsStream {
            cfg: self.cfg.clone(),
            inner: stream,
        })
    }
}

fn make_cfg() -> Arc<ServerConfig> {
    let f_cert = File::open("examples/rustls/cert.pem").unwrap();

    let mut reader = BufReader::new(f_cert);
    let cert = rustls_pemfile::certs(&mut reader).unwrap();
    let cert = cert[0].clone();

    let f_cert = File::open("examples/rustls/key.rsa").unwrap();
    let mut reader = BufReader::new(f_cert);
    let pris = rustls_pemfile::pkcs8_private_keys(&mut reader).unwrap();
    let pri = pris[0].clone();

    let private_key = PrivateKey(pri); //private.pem
    let cert = Certificate(cert); //cert

    let config = ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![cert], private_key)
        .unwrap();
    let cfg = Arc::new(config);
    cfg
}

fn main() {
    let _ = fast_log::init(Config::new().console());
    let ssl = RustlsSSL {
        cfg: make_cfg(),
    };
    let _listening = mco_http::Server::https("0.0.0.0:3000", ssl).unwrap()
        .handle(|req:Request,resp:Response|{
            resp.send(b"Hello World!").unwrap();
        });
    println!("Listening on http://127.0.0.1:3000");
}
