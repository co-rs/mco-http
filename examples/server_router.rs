use std::sync::Arc;

use cogo_http::route::Route;
use cogo_http::server::{Request, Response};
use cogo_http::route::MiddleWare;

fn main() {
    env_logger::init().unwrap();

    let mut route = Route::new();
    route.handle_fn("/", |req: Request, res: Response| {
        res.send(b"Hello World!").unwrap();
    });
    route.handle_fn("/js", |req: Request, res: Response| {
        res.send("{\"name\":\"joe\"}".as_bytes()).unwrap();
    });
    route.handle_fn("/fn", |req: Request, res: Response| {
        res.send(format!("fn").as_bytes());
    });

    let route = Arc::new(route);
    let _listening = cogo_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(route.clone());
    println!("Listening on http://127.0.0.1:3000");

    for x in &route.handlers {
        println!("please click http://127.0.0.1:3000{}",x.0);
    }
}