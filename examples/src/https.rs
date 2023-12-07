#[deny(unused_variables)]
extern crate mco_http;
extern crate fast_log;

use std::fs::File;
use std::io::Read;
use fast_log::config::Config;
use mco_http::server::{Request, Response};
use mco_http_rustls::TlsServer;


fn main() {
    let _ = fast_log::init(Config::new().console());

    let mut f_cert = File::open("examples/rustls/cert.pem").unwrap();
    let mut f_key = File::open("examples/rustls/key.rsa").unwrap();

    let mut buf =vec![];
    _ = f_cert.read_to_end(&mut buf);

    let mut buf2 =vec![];
    _ = f_key.read_to_end(&mut buf2);

    let ssl = TlsServer::new(vec![buf],buf2);

    let _listening = mco_http::Server::https("0.0.0.0:3000", ssl).unwrap()
        .handle(|req:Request,resp:Response|{
            resp.send(b"Hello World!").unwrap();
        });
    println!("Listening on https://127.0.0.1:3000");
}
