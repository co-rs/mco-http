#[deny(unused_variables)]
extern crate cogo_http;
extern crate env_logger;

use cogo_http::query::read_query;
use cogo_http::server::{Request, Response};

// http://127.0.0.1:3000?q=query_info
fn hello(req: Request, res: Response) {
    let q = read_query(&req.uri.to_string()).unwrap();
    let param =  q.get("q");
    res.send(param.unwrap_or(&"not find param q".to_string()).as_bytes()).unwrap();
}

fn main() {
    env_logger::init().unwrap();
    let _listening = cogo_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello);
    println!("Listening on http://127.0.0.1:3000");
    println!("please click http://127.0.0.1:3000?q=query_info");
}
