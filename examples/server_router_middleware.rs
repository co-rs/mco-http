use std::sync::Arc;
use mco_http::route::Route;
use mco_http::server::{Request, Response};
use mco_http::route::MiddleWare;

// MiddleWare
#[derive(Debug)]
pub struct MyMiddleWare {}

impl MiddleWare for MyMiddleWare {
    fn handle(&self, req: &mut Request, res: &mut Option<Response>) {
        println!("hello MiddleWare!");
        //You can carry any data here
        req.extra.insert( "joe".to_string());
    }
}

fn main() {
    env_logger::init().unwrap();

    let mut route = Route::new();
    route.add_middleware(MyMiddleWare {});
    route.handle_fn("/", |req: Request, res: Response| {
        res.send(format!("read from middleware: {:?}", req.extra.get::<String>()).as_bytes());
    });

    let route = Arc::new(route);
    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(route.clone());
    println!("Listening on http://127.0.0.1:3000");
}