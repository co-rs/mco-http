use std::sync::Arc;
use mco_http::route::{Route};
use mco_http::server::{Request, Response};
use mco_http::route::MiddleWare;
use fast_log::config::Config;

pub trait Api {
    fn js(&self, req: Request, res: Response);
}

impl Api for Route {
    fn js(&self, req: Request, mut res: Response) {
        let name = self.index::<&str>("name");
        let age = self.index::<i32>("age");
        res.send(format!("server name:{},age:{}", name, age).as_bytes()).unwrap();
    }
}


fn main() {
    let _=fast_log::init(Config::new().console());

    let mut route = Arc::new(Route::new());
    route.insert("name", "joe");
    route.insert("age", 18);

    let route_clone = route.clone();
    route.handle_fn("/", move |req: Request, res: Response| {
        route_clone.js(req, res);
    });


    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(route);
    println!("Listening on http://127.0.0.1:3000");
}