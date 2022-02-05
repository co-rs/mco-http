#[deny(unused_variables)]
extern crate cogo_http;
extern crate env_logger;
extern crate serde_json;

use std::fs::File;
use std::io::{BufWriter, Read};
use cogo_http::multipart::mult_part::read_formdata;
use cogo_http::server::{Request, Response};


// request header Content-Type: form-data
fn hello(mut req: Request, res: Response) {
    let form = read_formdata(&mut req.body, &req.headers, Some(|name| {
        //if upload file
        let path= format!("target/{}",name);
        let mut f = File::create(&path).unwrap();
        println!("will write file: {}",&path);
        Box::new(f)
    })).unwrap();
    res.send(serde_json::json!(form.fields).to_string().as_bytes()).unwrap();
}

fn main() {
    env_logger::init().unwrap();
    let _listening = cogo_http::Server::http("0.0.0.0:3000").unwrap()
        .handle_stack(hello,0x1000).unwrap();
    println!("Listening on http://127.0.0.1:3000");
}
