#[deny(unused_variables)]
extern crate cogo_http;
extern crate env_logger;
extern crate serde_json;

use std::io::Read;
use cogo_http::server::{Request, Response};


// request header Content-Type: json
fn hello(mut req: Request, res: Response) {
    println!("---------req----------\n {} \n-----------", req.headers);

    let mut json_data = String::new();
    req.read_to_string(&mut json_data);

    let result:serde_json::Value = serde_json::from_str(&json_data).unwrap_or_default();

    res.send(result.to_string().as_bytes()).unwrap();
}

fn main() {
    env_logger::init().unwrap();
    let _listening = cogo_http::Server::http("0.0.0.0:8001").unwrap()
        .handle(hello).unwrap();
    println!("Listening on http://127.0.0.1:8001");
}
