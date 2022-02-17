use std::any::Any;
use std::collections::HashMap;
use std::ops::Deref;

pub struct Extra {
    pub inner: HashMap<String, Box<dyn Any>>,
}

impl Extra {

    pub fn insert<T>(&mut self, k: String, value: T) -> Option<T> where T:'static {
        match self.inner.insert(k, Box::new(value)) {
            None => { None }
            Some(v) => {
                Some(*v.downcast::<T>().unwrap())
            }
        }
    }

    pub fn get<T>(&self, k: &str) -> Option<&T> where T:Any {
        match self.inner.get(k) {
            None => { None }
            Some(v) => {
                v.downcast_ref()
            }
        }
    }

    pub fn get_mut<T>(&mut self, k: &str) -> Option<&mut T> where T:Any {
        match self.inner.get_mut(k) {
            None => { None }
            Some(v) => {
                v.downcast_mut()
            }
        }
    }
}

impl Default for Extra{
    fn default() -> Self {
        Self{
            inner:HashMap::new()
        }
    }
}