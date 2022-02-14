#[deny(unused_variables)]
extern crate cogo_http;
extern crate env_logger;

use cogo_http::path::Path;
use cogo_http::query::read_query;
use cogo_http::server::{Request, Response};

// http://127.0.0.1:3000/1/2/3
fn hello(req: Request, res: Response) {
    let p = Path::new("/{a}/{b}/");
    let param =  p.read_path(&req.uri.to_string());
    res.send(format!("{:?}",param).as_bytes()).unwrap();
}

fn main() {
    env_logger::init().unwrap();
    let _listening = cogo_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello);
    println!("Listening on http://127.0.0.1:3000");
    println!("please click http://127.0.0.1:3000/1/2/3");
}
