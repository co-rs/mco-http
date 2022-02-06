use cogo_http::query::read_query;
use cogo_http::route::Route;
use cogo_http::server::{Request, Response};
use cogo_http::route::MiddleWare;

fn js(req: Request, res: Response) {
    res.send("{\"name\":\"joe\"}".as_bytes()).unwrap();
}

fn hello(req: Request, res: Response) {
    let param = read_query(&req.uri.to_string());
    res.send(format!("{:?}", param).as_bytes()).unwrap();
}

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
    route.handle_fn("/", hello);
    route.handle_fn("/js", js);
    route.handle_fn("/fn", |req: Request, res: Response| {
        res.send(format!("fn").as_bytes());
    });

    let _listening = cogo_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(route);
    println!("Listening on http://127.0.0.1:3000");
    println!("please click http://127.0.0.1:3000/?q=query_info&b=123");
    println!("please click http://127.0.0.1:3000/js");
    println!("please click http://127.0.0.1:3000/fn");
}