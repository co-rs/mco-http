#[deny(unused_variables)]
extern crate mco_http;
extern crate fast_log;
#[macro_use]
extern crate serde_json;
use fast_log::config::Config;
use mco_http::json::read_json;
use mco_http::server::{Request, Response};

// request header Content-Type: json
fn hello(mut req: Request, res: Response) {
    let json_data: serde_json::Value = read_json(&mut req).unwrap();
    println!("req:{:?}",json_data);
    res.send(json_data.to_string().as_bytes()).unwrap();
}

fn main() {
    fast_log::init(Config::new().console());
    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello).unwrap();
    println!("Listening on http://127.0.0.1:3000");
}
