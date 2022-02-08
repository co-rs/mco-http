#[deny(unused_variables)]
extern crate cogo_http;
extern crate env_logger;

use cogo_http::route::Route;
use cogo_http::server::{Request, Response};

fn hello(req: Request, res: Response) {
    res.send(b"Hello World!").unwrap();
}

fn main() {
    env_logger::init().unwrap();

    let _listening = cogo_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello);
    println!("Listening on http://127.0.0.1:3000");
}
