use cogo_http::net::Fresh;
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
pub struct MyMiddleWare {}

impl MiddleWare for MyMiddleWare {
    fn handle<'r, 'a, 'k>(&'r self, req: &'r mut Request<'a, 'k>, res: &'r mut Response<'a, Fresh>) -> bool {
        println!("hello MiddleWare!");
        return false;//return true is done
    }
}

fn main() {
    env_logger::init().unwrap();

    let mut route = Route::new();
    route.add_middleware(MyMiddleWare {});
    route.handle_fn("/", hello);
    route.handle_fn("/js", js);

    let _listening = cogo_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(route);
    println!("Listening on http://127.0.0.1:3000");
    println!("please click http://127.0.0.1:3000/?q=query_info&b=123");
}