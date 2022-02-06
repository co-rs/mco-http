use std::collections::BTreeMap;
use crate::error::Result;

pub fn read_query(url: &str) -> Result<BTreeMap<String, String>> {
    let mut m = BTreeMap::new();
    let idx= url.find("?");
    if idx.is_some(){
        let data: Vec<(String, String)> = serde_urlencoded::from_str(&url[idx.unwrap()+1..])?;
        for (k, v) in data {
            m.insert(k, v);
        }
    }
    return Ok(m);
}