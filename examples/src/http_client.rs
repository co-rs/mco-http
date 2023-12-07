extern crate mco_http;
extern crate fast_log;

use std::io;
use fast_log::config::Config;

use mco_http::Client;
use mco_http::header::Connection;

fn main() {
    let _ = fast_log::init(Config::new().level(log::LevelFilter::Info).console());

    let url = "http://127.0.0.1:3000".to_string();

    let client = Client::new();

    let mut res = client.get(&url)
        .header(Connection::close())
        .send().unwrap();

    println!("Response: {}", res.status);
    println!("Headers:\n{}", res.headers);
    io::copy(&mut res, &mut io::stdout()).unwrap();
}
