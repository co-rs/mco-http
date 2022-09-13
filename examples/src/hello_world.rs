#[deny(unused_variables)]
extern crate mco_http;
extern crate fast_log;

use std::io::{Read, Write};
use fast_log::config::Config;
use httparse::Status;
use mco_http::route::Route;
use mco_http::runtime;
use mco_http::runtime::{spawn_stack_size, TcpListener};
use mco_http::server::{Request, Response};

fn hello(req: Request, res: Response) {
    res.send(b"Hello World!").unwrap();
}

fn req_done(buf: &[u8], path: &mut String) -> Option<usize> {
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);

    if let Ok(Status::Complete(i)) = req.parse(buf) {
        path.clear();
        path.push_str(req.path.unwrap_or("/"));
        return Some(i);
    }

    None
}

fn main() {
    let _=fast_log::init(Config::new().console());
    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello);
    println!("Listening on http://127.0.0.1:3000");
}
