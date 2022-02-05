extern crate cogo_http;
extern crate env_logger;

use std::env;
use std::io;

use cogo_http::Client;
use cogo_http::header::Connection;

fn main() {
    env_logger::init().unwrap();

    let mut url = "http://www.baidu.com".to_string();

    let client = Client::new();

    let mut res = client.get(&url)
        .header(Connection::close())
        .send().unwrap();

    println!("Response: {}", res.status);
    println!("Headers:\n{}", res.headers);
    io::copy(&mut res, &mut io::stdout()).unwrap();
}
