#[deny(unused_variables)]
extern crate mco_http;
extern crate fast_log;
use fast_log::config::Config;
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
    let _=fast_log::init(Config::new().level(log::LevelFilter::Info).console());
    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello);
    println!("Listening on http://127.0.0.1:3000");
    println!("please click http://127.0.0.1:3000/1/2/3");
}
