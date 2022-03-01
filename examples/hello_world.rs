#[deny(unused_variables)]
extern crate mco_http;
extern crate fast_log;

use fast_log::config::Config;
use mco_http::route::Route;
use mco_http::server::{Request, Response};

fn hello(req: Request, res: Response) {
    res.send(b"Hello World!").unwrap();
}

fn main() {
    fast_log::init(Config::new().console());

    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello);
    println!("Listening on http://127.0.0.1:3000");
}
