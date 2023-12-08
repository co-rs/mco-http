extern crate mco_http;
extern crate fast_log;

use std::io;
use mco_http::Client;
use mco_http::net::HttpsConnector;
use mco_http_rustls::TlsClient;

fn main() {

    // //notice:    https_server.rs must use this code
    // let url = "https://localhost:3000".to_string();
    // use std::io::{BufReader, Cursor, Read};
    // let mut key=std::fs::File::open("examples/rustls/sample.pem").unwrap();
    // let mut buf=vec![];
    // _ = key.read_to_end(&mut buf);
    // let reader = &mut BufReader::new(Cursor::new(buf));
    // let client = Client::with_connector(HttpsConnector::new(TlsClient::new_ca(Some(reader)).unwrap()));


    let url = "https://www.baidu.com".to_string();
    let client = Client::with_connector(HttpsConnector::new(TlsClient::new()));

    let mut res = client.get(&url)
        .send().unwrap();

    println!("Response: {}", res.status);
    println!("Headers:\n{}", res.headers);
    io::copy(&mut res, &mut io::stdout()).unwrap();
}
