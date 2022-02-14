use std::collections::{BTreeMap};
use crate::error::Result;

#[derive(Clone,Debug)]
pub struct Path {
    // /{a}/{b}/{c}
    pub url: String,
    //key-value
    pub map: Vec<(i32, String)>,
}

impl Path {
    pub fn new(url: &str) -> Self {
        let url = &url[0..url.find("?").unwrap_or(url.len())];
        let args: Vec<&str> = url.split("/").collect();
        let mut map = Vec::new();
        let mut idx = 0;
        for x in args {
            if x.starts_with("{") && x.ends_with("}") {
                map.push((idx, x.trim_start_matches("{").trim_end_matches("}").to_string()));
            }
            idx += 1;
        }
        Self {
            url: url.to_string(),
            map: map,
        }
    }

    pub fn read_path(&self, url: &str) -> BTreeMap<String, String> {
        let url = &url[0..url.find("?").unwrap_or(url.len())];
        let args: Vec<&str> = url.split("/").collect();
        let mut params = BTreeMap::new();
        let mut idx = 0;
        for x in args {
            for (i, v) in &self.map {
                if *i == idx {
                    params.insert(v.to_string(), x.to_string());
                }
            }
            idx += 1;
        }
        params
    }
}