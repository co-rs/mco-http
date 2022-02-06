use std::sync::Arc;
use cogo_http::route::{Route};
use cogo_http::server::{Request, Response};
use cogo_http::route::MiddleWare;


pub trait Api {
    fn js(&self, req: Request, res: Response);
}

impl Api for Route {
    fn js(&self, req: Request, res: Response) {
        let name = self.get::<&str>("name");
        res.send(format!("server name:{:?}", name).as_bytes()).unwrap();
    }
}


fn main() {
    env_logger::init().unwrap();

    let mut route = Arc::new(Route::new());
    route.insert("name", "joe");

    let route_clone = route.clone();
    route.handle_fn("/", move |req: Request, res: Response| {
        route_clone.js(req, res);
    });


    let _listening = cogo_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(route);
    println!("Listening on http://127.0.0.1:3000");
}