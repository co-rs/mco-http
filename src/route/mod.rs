use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{Debug, Formatter};
use crate::net::Fresh;
use crate::server::{Handler, Request, Response};
use crate::status::StatusCode;
use crate::uri::RequestUri::AbsolutePath;
use std::io::copy;
use crate::uri::RequestUri;

pub struct HandleBox {
    pub url: String,
    pub inner: Box<dyn Handler>,
}

impl Debug for HandleBox{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandleBox")
            .field("url",&self.url)
            .field("inner",&"*")
            .finish()
    }
}


pub trait Container: Any + Send + Sync + Debug {}


pub trait MiddleWare: Send + Sync+Debug {
    //return is finish. if finish will be return
    fn handle(&self, req: &mut Request<'_, '_>, res: &mut Response<'_,Fresh>) -> bool;
}

#[derive(Debug)]
pub struct Route {
    pub container: BTreeMap<String, Box<dyn Container>>,
    pub middleware: Vec<Box<dyn MiddleWare>>,
    pub handlers: HashMap<String, HandleBox>,
}

impl Route {
    pub fn new() -> Self {
        Self {
            container: Default::default(),
            middleware: vec![],
            handlers: Default::default(),
        }
    }
    pub fn handle_fn<H: Handler + 'static>(&mut self, url: &str, h: H) {
        self.handlers.insert(url.to_string(), HandleBox {
            url: url.to_string(),
            inner: Box::new(h),
        });
    }

    pub fn add_middleware<M: MiddleWare+'static>(&mut self,m:M){
        self.middleware.push(Box::new(m));
    }
}


impl Handler for Route {
    fn handle<'a, 'k>(&'a self, mut req: Request<'a, 'k>, mut res: Response<'a, Fresh>) {
        for x in &self.middleware {
            //finish?.this is safety
            if x.handle(&mut req,  &mut res) {
                return;
            }
        }
        match &req.uri {
            AbsolutePath(p) => {
                match self.handlers.get(&p[0..p.find("?").unwrap_or(p.len())]) {
                    None => {
                        //404
                        res.status = StatusCode::NotFound;
                        return;
                    }
                    Some(h) => {
                        h.inner.handle(req, res);
                        return;
                    }
                }
            }
            RequestUri::AbsoluteUri(_) => {}
            RequestUri::Authority(_) => {}
            RequestUri::Star => {}
        }
    }
}