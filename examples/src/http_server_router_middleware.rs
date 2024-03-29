use std::sync::Arc;
use mco_http::route::Route;
use mco_http::server::{Request, Response};
use mco_http::route::MiddleWare;
use fast_log::config::Config;

// MiddleWare
#[derive(Debug)]
pub struct MyMiddleWare {
    route: Arc<Route>,
}

impl MiddleWare for MyMiddleWare {
    fn handle(&self, req: &mut Request, res: &mut Option<Response>) {
        println!("hello MiddleWare!");
        //You can carry any data here
        req.extra.insert(self.route.clone());
        req.extra.insert("joe".to_string());
    }
}

fn main() {
    let _ = fast_log::init(Config::new().level(log::LevelFilter::Info).console());

    let mut route = Arc::new(Route::new());
    route.add_middleware(MyMiddleWare { route: route.clone() });
    route.add_middleware(|req: &mut Request, res: &mut Option<Response>| {});
    route.handle_fn("/", |req: Request, res: Response| {
        res.send(format!("read from middleware: {:?}", req.extra.get::<String>()).as_bytes());
    });

    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(route);
    println!("Listening on http://127.0.0.1:3000");
}
