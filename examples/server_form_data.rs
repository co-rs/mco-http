#[deny(unused_variables)]
extern crate mco_http;
extern crate env_logger;
extern crate serde_json;

use std::fs::File;
use std::io::{Read, Write};
use mco_http::multipart::mult_part::read_formdata;
use mco_http::server::{Request, Response};

fn hello(mut req: Request, res: Response) {
    let head= req.headers().clone();
    let form = read_formdata( req.body_mut(), &head,None).unwrap();
    res.send(serde_json::json!(form.fields).to_string().as_bytes()).unwrap();
}

fn main() {
    env_logger::init().unwrap();
    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello).unwrap();
    println!("Listening on http://127.0.0.1:3000");
}
