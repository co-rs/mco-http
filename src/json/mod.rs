use std::io;
use std::io::Read;
use serde::de::DeserializeOwned;
use crate::server::Request;
use crate::error::Result;

pub fn read_json<T: DeserializeOwned>(req: &mut Request) -> Result<T> {
    let mut json_data = Vec::new();
    req.read_to_end(&mut json_data)?;
    Ok(serde_json::from_slice(&json_data)?)
}