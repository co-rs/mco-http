extern crate mco_http;
extern crate fast_log;

use std::io;
use fast_log::config::Config;
use mco_http::Client;
use mco_http::net::HttpsConnector;
use mco_http_rustls::TlsClient;

fn main() {
    let url = "https://www.baidu.com".to_string();

    // let url = "https://127.0.0.1:3000".to_string();
    // use std::io::{BufReader, Cursor, Read};
    // let mut key=std::fs::File::open("examples/rustls/cert.pem").unwrap();
    // let mut buf=vec![];
    // _ = key.read_to_end(&mut buf);
    // let reader = &mut BufReader::new(Cursor::new(buf));
    // let client = Client::with_connector(HttpsConnector::new(TlsClient::new_ca(Some(reader)).unwrap()));

    let client = Client::with_connector(HttpsConnector::new(TlsClient::new()));

    let mut res = client.get(&url)
        .send().unwrap();

    println!("Response: {}", res.status);
    println!("Headers:\n{}", res.headers);
    io::copy(&mut res, &mut io::stdout()).unwrap();
}
