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
use dashmap::DashMap;
use crate::runtime::{SyncHashMap};
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
    pub container: SyncHashMap<String, Arc<Box<dyn Any>>>,
    pub middleware: Vec<Box<dyn MiddleWare>>,
    pub handlers: SyncHashMap<String, HandleBox>,
}


impl Route {
    pub fn new() -> Self {
        Self {
            container: SyncHashMap::new(),
            middleware: vec![],
            handlers: SyncHashMap::new(),
        }
    }
    pub fn handle_fn<H: Handler + 'static>(&self, url: &str, h: H) {
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
    fn handle(&self, mut req: Request, mut res: Response<'_, Fresh>) {
        for x in &self.middleware {
            //finish?.this is safety
            if x.handle(&mut req, &mut res) {
                return;
            }
        }
        match &req.uri {
            AbsolutePath(p) => {
                let path = &p[0..p.find("?").unwrap_or(p.len())];
                match self.handlers.get(path) {
                    None => {
                        //404
                        res.status = StatusCode::NotFound;
                        return;
                    }
                    Some(h) => {
                        let i = &h.inner;
                        i.handle(req, res);
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

impl Handler for Arc<Route> {
    fn handle(&self, mut req: Request, mut res: Response<'_, Fresh>) {
        Route::handle(self, req, res)
    }
}