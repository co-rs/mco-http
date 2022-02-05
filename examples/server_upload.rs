#[deny(unused_variables)]
extern crate cogo_http;
extern crate env_logger;
extern crate serde_json;

use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use cogo_http::multipart::mult_part::read_formdata;
use cogo_http::server::{Request, Response};

// request header Content-Type: json
fn hello(mut req: Request, res: Response) {
    let form = read_formdata(&mut req.body, &req.headers, Some(|w| -> std::io::Result<()>{
        let path="target/".to_string() + &w.filename().unwrap_or("temp.file".to_string());
        w.set_write(File::create(&path)?);
        println!("upload: {}",w.key);
        println!("file name: {}",path);
        // w.path = PathBuf::new();
        // w.path.push(&path);
        Ok(())
    })).unwrap();
    res.send("upload success".to_string().as_bytes()).unwrap();
}

fn main() {
    env_logger::init().unwrap();
    let _listening = cogo_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello).unwrap();
    println!("Listening on http://127.0.0.1:3000");
}
