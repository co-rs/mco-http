#[deny(unused_variables)]
extern crate cogo_http;
extern crate env_logger;
extern crate serde_json;

use std::fs::File;
use std::io::{Read, Write};
use cogo_http::multipart::mult_part::read_formdata;
use cogo_http::server::{Request, Response};


// request header Content-Type: json
fn hello(mut req: Request, res: Response) {
    let mut files=vec![];
    let form = read_formdata(&mut req.body, &req.headers).unwrap();
    for (k, mut v) in form.files {
        if v.path.is_file() {
            v.do_not_delete_on_drop();
            let name = v.filename().unwrap_or_default();
            if !name.is_empty() {
                //rename
                std::fs::rename(v.path.to_str().unwrap(), "target/mime_multipart/".to_string() + &name);
                files.push(name);
            }
        }
    }
    res.send(format!("upload:{:?}",files).to_string().as_bytes()).unwrap();
}

fn main() {
    env_logger::init().unwrap();
    let _listening = cogo_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello).unwrap();
    println!("Listening on http://127.0.0.1:3000");
}
