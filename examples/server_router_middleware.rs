use std::sync::Arc;
use mco_http::query::read_query;
use mco_http::route::Route;
use mco_http::server::{Request, Response};
use mco_http::route::MiddleWare;

// MiddleWare
#[derive(Debug)]
pub struct MyMiddleWare {}

impl MiddleWare for MyMiddleWare {
    fn handle(&self, req: &mut Request, res: &mut Response) -> bool {
        println!("hello MiddleWare!");
        //return true is done
        return false;
    }
}

fn main() {
    env_logger::init().unwrap();

    let mut route = Route::new();
    route.add_middleware(MyMiddleWare {});
    route.handle_fn("/", |req: Request, res: Response| {
        let param = read_query(&req.uri.to_string());
        res.send(format!("{:?}", param).as_bytes()).unwrap();
    });

    let route = Arc::new(route);
    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(route.clone());
    println!("Listening on http://127.0.0.1:3000");

    for x in &route.handlers {
        println!("please click http://127.0.0.1:3000{}{}",x.0,{if x.0.eq("/"){ "?a=b&c=2" }else{ "" }});
    }
}