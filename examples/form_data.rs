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
    let form = read_formdata(&mut req.body, &req.headers).unwrap();
    for (k, v) in form.files {
        if v.path.is_file() {
            let mut r = File::open(v.path.to_str().unwrap_or_default()).unwrap();
            std::fs::create_dir_all("target/upload/");
            let name = "target/upload/".to_string() + &v.filename().unwrap_or_default();
            let mut f = File::create(&name).unwrap();
            std::io::copy(&mut r, &mut f);
        }
    }
    res.send(serde_json::json!(form.fields).to_string().as_bytes()).unwrap();
}

fn main() {
    env_logger::init().unwrap();
    let _listening = cogo_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello).unwrap();
    println!("Listening on http://127.0.0.1:3000");
}
