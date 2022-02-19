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
use std::rc::Rc;
use std::sync::Arc;
use mco::std::vec::SyncVec;
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

pub trait MiddleWare: Send + Sync {
    /// if you call take Response, next handle will be not run
    fn handle(&self, req: &mut Request, res: &mut Option<Response>);
}

impl<T: MiddleWare> MiddleWare for Arc<T> {
    fn handle(&self, req: &mut Request, res: &mut Option<Response>) {
        T::handle(self, req, res)
    }
}

impl<F> MiddleWare for F where F: Fn(&mut Request, &mut Option<Response>), F: Sync + Send {
    fn handle(&self, req: &mut Request, res: &mut Option<Response>) {
        self(req, res)
    }
}

pub struct Route {
    pub container: SyncHashMap<String, Arc<Box<dyn Any>>>,
    pub middleware: SyncVec<Box<dyn MiddleWare>>,
    pub handlers: SyncHashMap<String, HandleBox>,
}

impl Debug for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Route")
            .field("container", &self.container.len())
            .field("middleware", &self.middleware.len())
            .field("handlers", &self.handlers)
            .finish()
    }
}


impl Route {
    pub fn new() -> Self {
        Self {
            container: SyncHashMap::new(),
            middleware: SyncVec::new(),
            handlers: SyncHashMap::new(),
        }
    }
    /// handle a fn
    /// for example:
    /// ```rust
    /// use mco_http::route::Route;
    /// use mco_http::server::{Request, Response};
    ///
    /// let mut route = Route::new();
    /// //Common way
    /// route.handle_fn("/", |req: Request, res: Response| {
    ///         res.send(b"Hello World!").unwrap();
    ///     });
    ///
    /// //or you can use method. It can even nest calls to Handle
    /// fn hello(req: Request, res: Response) {
    ///     res.send(b"Hello World!").unwrap();
    /// }
    /// route.handle_fn("/",hello);
    ///
    ///
    /// ```
    pub fn handle_fn<H: Handler + 'static>(&self, url: &str, h: H) {
        self.handlers.insert(url.to_string(), HandleBox {
            url: url.to_string(),
            inner: Box::new(h),
        });
    }

    /// if you take Response. handle be done
    /// for example:
    /// ```rust
    /// use mco_http::server::{Request, Response};
    /// use mco_http::route::Route;
    /// let mut route = Route::new();
    /// route.add_middleware(|req: &mut Request, res: &mut Option<Response>|{
    ///        ///res.take() //take Response, next handle will be not run
    ///     });
    /// ```
    pub fn add_middleware<M: MiddleWare + 'static>(&self, m: M) {
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

    /// index will get from container.if not exist will be panic!
    pub fn index<T: Any>(&self, key: &str) -> &T {
        self.get(key).expect(&format!("key:{} Does not exist in the container", key))
    }
}


impl Handler for Route {
    fn handle(&self, mut req: Request, mut res: Response) {
        for  m in &self.middleware {
            let mut r = Some(res);
            //finish?.this is safety
            m.handle(&mut req, &mut r);
            if r.is_none() {
                return;
            } else {
                res = r.unwrap();
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