use std::sync::Arc;

use mco_http::route::Route;
use mco_http::server::{Request, Response};
use mco_http::route::MiddleWare;
use fast_log::config::Config;

fn main() {
    let _=fast_log::init(Config::new().level(log::LevelFilter::Info).console());

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
    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(route.clone());
    println!("Listening on http://127.0.0.1:3000");

    for x in &route.handlers {
        println!("please click http://127.0.0.1:3000{}",x.0);
    }
}