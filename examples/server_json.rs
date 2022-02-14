#[deny(unused_variables)]
extern crate cogo_http;
extern crate env_logger;
#[macro_use]
extern crate serde_json;

use cogo_http::json::read_json;
use cogo_http::server::{Request, Response};

// request header Content-Type: json
fn hello(mut req: Request, res: Response) {
    let json_data: serde_json::Value = read_json(&mut req).unwrap();
    println!("req:{:?}",json_data);
    res.send(json_data.to_string().as_bytes()).unwrap();
}

fn main() {
    env_logger::init().unwrap();
    let _listening = cogo_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello).unwrap();
    println!("Listening on http://127.0.0.1:3000");
}
