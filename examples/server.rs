#[deny(unused_variables)]
extern crate cogo_http;
extern crate env_logger;

use std::io::copy;

use cogo_http::{Get, Post};
use cogo_http::server::{Server, Request, Response};
use cogo_http::status::StatusCode;
use cogo_http::uri::RequestUri::AbsolutePath;

macro_rules! try_return(
    ($e:expr) => {{
        match $e {
            Ok(v) => v,
            Err(e) => { println!("Error: {}", e); return; }
        }
    }}
);

fn routing(mut req: Request, mut res: Response) {
    match req.uri {
        AbsolutePath(ref path) => match (&req.method, &path[..]) {
            (&Get, "/") | (&Get, "/echo") => {
                try_return!(res.send(b"echo"));
                return;
            },
            (&Post, "/echo") => (), // fall through, fighting mutable borrows
            _ => {
                *res.status_mut() = cogo_http::NotFound;
                return;
            }
        },
        _ => {
            res.status = StatusCode::NotFound;
            return;
        }
    };

    let mut res = try_return!(res.start());
    try_return!(copy(&mut req, &mut res));
}

fn main() {
    env_logger::init().unwrap();
    let server = Server::http("127.0.0.1:1337").unwrap();
    let _guard = server.handle(routing);
    println!("Listening on http://127.0.0.1:1337");
}
