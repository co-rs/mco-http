use std::str::FromStr;
use crate::route::Route;
use crate::server::{Request, Response};

/// proxy_localhost '/api/' to '/api/admin/'
pub fn proxy_localhost(router: &mut Route, url_from: &str, url_to: &str) {
    router.add_middleware(|req: &mut Request, res: &mut Option<Response>| {
        let uri = req.uri.to_string().replace(url_from, url_to).to_string();
        req.uri = crate::uri::RequestUri::from_str(&uri).unwrap();
    });
}