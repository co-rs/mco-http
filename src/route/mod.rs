use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt::{Debug, Formatter};
use crate::net::Fresh;
use crate::server::{Handler, Request, Response};
use crate::status::StatusCode;
use crate::uri::RequestUri::AbsolutePath;
use std::io::copy;
use std::ops::Deref;
use std::sync::Arc;
use crate::runtime::SyncBtreeMap;
use crate::uri::RequestUri;

pub struct HandleBox {
    pub url: String,
    pub inner: Box<dyn Handler>,
}

impl Debug for HandleBox {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandleBox")
            .field("url", &self.url)
            .field("inner", &"*")
            .finish()
    }
}


pub trait MiddleWare: Send + Sync + Debug {
    //return is finish. if finish will be return
    fn handle(&self, req: &mut Request, res: &mut Response) -> bool;
}

#[derive(Debug)]
pub struct Route {
    pub container: SyncBtreeMap<String, Arc<Box<dyn Any>>>,
    pub middleware: Vec<Box<dyn MiddleWare>>,
    pub handlers: HashMap<String, HandleBox>,
}

impl Route {
    pub fn new() -> Self {
        Self {
            container: SyncBtreeMap::new(),
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

    pub fn add_middleware<M: MiddleWare + 'static>(&mut self, m: M) {
        self.middleware.push(Box::new(m));
    }

    pub fn insert<T: Any>(&self, key: &str, data: T) {
        self.container.insert(key.to_string(), Arc::new(Box::new(data)));
    }

    pub fn get<T: Any>(&self, key: &str) -> Option<&T> {
        match self.container.get(key) {
            None => {
                None
            }
            Some(b) => {
                let r: Option<&T> = b.downcast_ref();
                r
            }
        }
    }
}


impl Handler for Route {
    fn handle<'a, 'k>(&'a self, mut req: Request<'a, 'k>, mut res: Response<'a, Fresh>) {
        for x in &self.middleware {
            //finish?.this is safety
            if x.handle(&mut req, &mut res) {
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