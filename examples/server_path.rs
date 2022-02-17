#[deny(unused_variables)]
extern crate mco_http;
extern crate env_logger;

use mco_http::path::Path;
use mco_http::query::read_query;
use mco_http::server::{Request, Response};

// http://127.0.0.1:3000/1/2/3
fn hello(req: Request, res: Response) {
    let p = Path::new("/{a}/{b}/");
    let param =  p.read_path(&req.uri.to_string());
    res.send(format!("{:?}",param).as_bytes()).unwrap();
}

fn main() {
    env_logger::init().unwrap();
    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello);
    println!("Listening on http://127.0.0.1:3000");
    println!("please click http://127.0.0.1:3000/1/2/3");
}
