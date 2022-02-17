extern crate env_logger;
extern crate mco_http;

use std::env;
use std::io;
use mco_http::Client;

fn main() {
    env_logger::init();

    let mut url = "http://www.baidu.com".to_string();

    let client = Client::new();

    let mut res = client.get(&url)
        .send().unwrap();

    println!("Response: {}", res.status);
    println!("Headers:\n{:?}", res.headers);
    io::copy(&mut res, &mut io::stdout()).unwrap();
}
