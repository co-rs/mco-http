use std::io::{ErrorKind, Read, Write};
use http::{HeaderMap, HeaderValue};
use crate::multipart::{Node, Part, FilePart};
use mime::{Mime, TopLevel, SubLevel};
use crate::multipart::error::Error;

/// The extracted text fields and uploaded files from a `multipart/form-data` request.
///
/// Use `parse_multipart` to devise this object from a request.
#[derive(Clone, Debug)]
pub struct FormData {
    /// Name-value pairs for plain text fields. Technically, these are form data parts with no
    /// filename specified in the part's `Content-Disposition`.
    pub fields: Vec<(String, String)>,
    /// Name-value pairs for temporary files. Technically, these are form data parts with a filename
    /// specified in the part's `Content-Disposition`.
    pub files: Vec<(String, FilePart)>,
}

impl FormData {
    pub fn new() -> FormData {
        FormData { fields: vec![], files: vec![] }
    }

    /// Create a mime-multipart Vec<Node> from this FormData
    pub fn to_multipart(&mut self) -> Result<Vec<Node>, Error> {
        // Translate to Nodes
        let mut nodes: Vec<Node> = Vec::with_capacity(self.fields.len() + self.files.len());

        for &(ref name, ref value) in &self.fields {
            let mut h = http::HeaderMap::with_capacity(2);
            h.insert(http::header::CONTENT_TYPE,HeaderValue::from_str(&Mime(TopLevel::Text, SubLevel::Plain, vec![]).to_string()).unwrap());
            h.insert(http::header::CONTENT_DISPOSITION,HeaderValue::from_str(&format!("form-data;name={}",name)).unwrap());
            nodes.push(Node::Part(Part {
                headers: h,
                body: value.as_bytes().to_owned(),
            }));
        }

        for &(ref name, ref filepart) in &self.files {
            let mut filepart = filepart.clone();
            // We leave all headers that the caller specified, except that we rewrite
            // Content-Disposition.
            filepart.headers.remove(http::header::CONTENT_DISPOSITION);
            let filename = match filepart.filename() {
                Ok(fname) => fname.to_string(),
                Err(_) => return Err(Error::Io(std::io::Error::new(ErrorKind::InvalidData, "not a file"))),
            };
            filepart.headers.insert(http::header::CONTENT_DISPOSITION,HeaderValue::from_str(&format!("form-data;name={};filename={}",name,filename)).unwrap());
            nodes.push(Node::File(filepart));
        }

        Ok(nodes)
    }
}


/// Parse MIME `multipart/form-data` information from a stream as a `FormData`.
pub fn read_formdata<S: Read>(headers: &HeaderMap,stream:  S, f: Option<fn(name: &mut FilePart) -> std::io::Result<()>>) -> Result<FormData, Error>
{
    let nodes = crate::multipart::read_multipart_body(stream, headers, false, f)?;
    let mut formdata = FormData::new();
    fill_formdata(&mut formdata, nodes)?;
    Ok(formdata)
}

// order and nesting are irrelevant, so we interate through the nodes and put them
// into one of two buckets (fields and files);  If a multipart node is found, it uses
// the name in its headers as the key (rather than the name in the headers of the
// subparts), which is how multiple file uploads work.
fn fill_formdata(formdata: &mut FormData, nodes: Vec<Node>) -> Result<(), Error>
{
    for node in nodes {
        match node {
            Node::Part(part) => {
                let cd_name: Option<String> = {
                    let cd = match part.headers.get(http::header::CONTENT_DISPOSITION) {
                        Some(cd) => cd,
                        None => return Err(Error::MissingDisposition),
                    };
                    get_content_disposition_name(cd)
                };
                let key = cd_name.ok_or(Error::NoName)?;
                let val = String::from_utf8(part.body)?;
                formdata.fields.push((key, val));
            }
            Node::File(part) => {
                let cd_name: Option<String> = {
                    let cd = match part.headers.get(http::header::CONTENT_DISPOSITION) {
                        Some(cd) => cd,
                        None => return Err(Error::MissingDisposition),
                    };
                    get_content_disposition_name(&cd)
                };
                let key = cd_name.ok_or(Error::NoName)?;
                formdata.files.push((key, part));
            }
            Node::Multipart((headers, nodes)) => {
                let cd_name: Option<String> = {
                    let cd = match headers.get(http::header::CONTENT_DISPOSITION) {
                        Some(cd) => cd,
                        None => return Err(Error::MissingDisposition),
                    };
                    get_content_disposition_name(&cd)
                };
                let key = cd_name.ok_or(Error::NoName)?;
                for node in nodes {
                    match node {
                        Node::Part(part) => {
                            let val = String::from_utf8(part.body)?;
                            formdata.fields.push((key.clone(), val));
                        }
                        Node::File(part) => {
                            formdata.files.push((key.clone(), part));
                        }
                        _ => {} // don't recurse deeper
                    }
                }
            }
        }
    }
    Ok(())
}

//Disposition
#[inline]
pub fn get_content_disposition_name(cd: &http::HeaderValue) -> Option<String> {

    let vec:Vec<&str>= cd.to_str().unwrap_or_default().split(";").collect();
    let mut idx =0;
    for x in &vec {
        if (*x).eq("name"){
            return match vec.get(idx+1){
                None => {
                    None
                }
                Some(v) => {
                    Some(v.to_string())
                }
            };
        }
        idx+=1;
    }
    return None;
}


/// Stream out `multipart/form-data` body content matching the passed in `formdata`.  This
/// does not stream out headers, so the caller must stream those out before calling
/// write_formdata().
pub fn write_formdata<S: Write,W:Write>(stream: &mut S, boundary: &Vec<u8>, formdata: &mut FormData, w: Option<fn(name: &mut FilePart) -> std::io::Result<()>>)
                                        -> Result<usize, Error>
{
    let mut nodes = formdata.to_multipart()?;

    // Write out
    let count = crate::multipart::write_multipart(stream, boundary, &mut nodes,w)?;

    Ok(count)
}

/// Stream out `multipart/form-data` body content matching the passed in `formdata` as
/// Transfer-Encoding: Chunked.  This does not stream out headers, so the caller must stream
/// those out before calling write_formdata().
pub fn write_formdata_chunked<S: Write>(stream: &mut S, boundary: &Vec<u8>, formdata: &mut FormData)
                                        -> Result<(), Error>
{
    let nodes = formdata.to_multipart()?;

    // Write out
    crate::multipart::write_multipart_chunked(stream, boundary, &nodes)?;

    Ok(())
}
