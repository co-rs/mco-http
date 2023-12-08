#[deny(unused_variables)]
extern crate mco_http;
extern crate fast_log;
extern crate serde_json;

use fast_log::config::Config;
use std::fs::File;
use std::io::{Read, Write};
use mco_http::multipart::mult_part::read_formdata;
use mco_http::server::{Request, Response};

fn hello(mut req: Request, res: Response) {
    let form = read_formdata(&mut req.body, &req.headers, None).unwrap();
    res.send(serde_json::json!(form.fields).to_string().as_bytes()).unwrap();
}

fn main() {
    let _ = fast_log::init(Config::new().level(log::LevelFilter::Info).console());
    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello).unwrap();
    println!("Listening on http://127.0.0.1:3000");
}
