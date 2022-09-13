use std::collections::BTreeMap;
pub fn read_query(url: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    let idx = url.find("?");
    if idx.is_some() {
        let data: Vec<(String, String)> = serde_urlencoded::from_str(&url[idx.unwrap() + 1..]).unwrap_or_default();
        for (k, v) in data {
            m.insert(k, v);
        }
    }
    return m;
}