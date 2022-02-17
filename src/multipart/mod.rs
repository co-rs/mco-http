// Copyright 2016-2020 mime-multipart Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

pub mod error;
pub mod mult_part;
pub mod byte_buf;

mod charset;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use error::Error;

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::borrow::{BorrowMut, Cow};
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::ops::{DerefMut, Drop};
use std::str::FromStr;
use std::sync::Arc;
use encoding::{all, Encoding, DecoderTrap};
use textnonce::TextNonce;
use mime::{Attr, Mime, TopLevel, Value};
use buf_read_ext::BufReadExt;
use http::header::{HeaderName, InvalidHeaderName};
use http::HeaderValue;
use charset::*;
use crate::{header_name, header_value};

pub trait ReadWrite: Write + Read {}

impl<T> ReadWrite for T where T: Read + Write {}

/// A multipart part which is not a file (stored in memory)
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub headers: http::HeaderMap,
    pub body: Vec<u8>,
}

impl Part {
    /// Mime content-type specified in the header
    pub fn content_type(&self) -> Option<Mime> {
        let ct = self.headers.get(http::header::CONTENT_TYPE);
        ct.map(|ref ct| {
            Mime::from_str(ct.to_str().unwrap_or_default()).unwrap()
        })
    }
}

/// A file that is to be inserted into a `multipart/*` or alternatively an uploaded file that
/// was received as part of `multipart/*` parsing.
pub struct FilePart {
    /// The headers of the part
    pub headers: http::HeaderMap,
    /// Optionally, the size of the file.  This is filled when multiparts are parsed, but is
    /// not necessary when they are generated.
    pub size: Option<usize>,

    pub path: PathBuf,

    pub key: String,

    pub write: Option<Box<dyn ReadWrite>>,
}

impl Clone for FilePart {
    fn clone(&self) -> Self {
        Self {
            headers: self.headers.clone(),
            size: self.size.clone(),
            path: Default::default(),
            key: "".to_string(),
            write: None,
        }
    }
}

impl Debug for FilePart {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilePart")
            .field("headers", &self.headers)
            .field("size", &self.size)
            .finish()
    }
}

impl FilePart {
    pub fn new(headers: http::HeaderMap, path: PathBuf) -> FilePart
    {
        FilePart {
            headers: headers,
            size: None,
            path: path,
            key: "".to_string(),
            write: None,
        }
    }

    /// set any Write and Read impl struct to FilePart
    pub fn set_write<W: ReadWrite + 'static>(&mut self, w: W) {
        self.write = Some(Box::new(w));
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }

    /// Create a new temporary FilePart (when created this way, the file will be
    /// deleted once the FilePart object goes out of scope).
    pub fn create(headers: http::HeaderMap) -> Result<FilePart, Error> {
        let cd_name: Option<String> = {
            let cd = match headers.get(http::header::CONTENT_DISPOSITION) {
                Some(cd) => cd,
                None => return Err(Error::MissingDisposition),
            };
            crate::multipart::mult_part::get_content_disposition_name(&cd)
        };
        Ok(FilePart {
            headers: headers,
            size: None,
            path: Default::default(),
            key: cd_name.unwrap_or_default(),
            write: None,
        })
    }

    /// Filename that was specified when the file was uploaded.  Returns `Ok<None>` if there
    /// was no content-disposition header supplied.
    pub fn filename(&self) -> Result<String, Error> {
        let cd = self.headers.get(http::header::CONTENT_DISPOSITION);
        match cd {
            Some(cd) => match get_content_disposition_filename(Some(&cd)) {
                Ok(v) => { Ok(v.unwrap()) }
                Err(e) => { Err(e) }
            },
            None => Err(Error::NoName),
        }
    }

    /// Mime content-type specified in the header
    pub fn content_type(&self) -> Option<Mime> {
        let ct = self.headers.get(http::header::CONTENT_TYPE);
        ct.map(|ref ct| {
            Mime::from_str(ct.to_str().unwrap_or_default()).unwrap()
        })
    }
}

/// A multipart part which could be either a file, in memory, or another multipart
/// container containing nested parts.
#[derive(Clone, Debug)]
pub enum Node {
    /// A part in memory
    Part(Part),
    /// A part streamed to a file
    File(FilePart),
    /// A container of nested multipart parts
    Multipart((http::HeaderMap, Vec<Node>)),
}

/// Parse a MIME `multipart/*` from a `Read`able stream into a `Vec` of `Node`s, streaming
/// files to disk and keeping the rest in memory.  Recursive `multipart/*` parts will are
/// parsed as well and returned within a `Node::Multipart` variant.
///
/// If `always_use_files` is true, all parts will be streamed to files.  If false, only parts
/// with a `ContentDisposition` header set to `Attachment` or otherwise containing a `Filename`
/// parameter will be streamed to files.
///
/// It is presumed that the headers are still in the stream.  If you have them separately,
/// use `read_multipart_body()` instead.
pub fn read_multipart<S: Read>(
    stream: &mut S,
    always_use_files: bool, f: Option<fn(name: &mut FilePart) -> std::io::Result<()>>)
    -> Result<Vec<Node>, Error>
{
    let mut reader = BufReader::with_capacity(4096, stream);
    let mut nodes: Vec<Node> = Vec::new();

    let mut buf: Vec<u8> = Vec::new();

    let (_, found) = reader.stream_until_token(b"\r\n\r\n", &mut buf)?;
    if !found { return Err(Error::EofInMainHeaders); }

    // Keep the CRLFCRLF as httparse will expect it
    buf.extend(b"\r\n\r\n".iter().cloned());

    // Parse the headers
    let mut header_memory = [httparse::EMPTY_HEADER; 64];
    let headers = match httparse::parse_headers(&buf, &mut header_memory) {
        Ok(httparse::Status::Complete((_, raw_headers))) => {
            let mut h = http::HeaderMap::new();
            for x in raw_headers {
                h.insert(header_name!(x.name)?, header_value!(&String::from_utf8(x.value.to_vec()).unwrap_or_default()));
            }
            Ok(h)
        }
        Ok(httparse::Status::Partial) => Err(Error::PartialHeaders),
        Err(err) => Err(From::from(err)),
    }?;

    inner(&mut reader, &headers, &mut nodes, always_use_files, f)?;
    Ok(nodes)
}

/// Parse a MIME `multipart/*` from a `Read`able stream into a `Vec` of `Node`s, streaming
/// files to disk and keeping the rest in memory.  Recursive `multipart/*` parts will are
/// parsed as well and returned within a `Node::Multipart` variant.
///
/// If `always_use_files` is true, all parts will be streamed to files.  If false, only parts
/// with a `ContentDisposition` header set to `Attachment` or otherwise containing a `Filename`
/// parameter will be streamed to files.
///
/// It is presumed that you have the `Headers` already and the stream starts at the body.
/// If the headers are still in the stream, use `read_multipart()` instead.
pub fn read_multipart_body<S: BufRead>(
    stream:  &mut S,
    headers: &http::HeaderMap,
    always_use_files: bool, f: Option<fn(name: &mut FilePart) -> std::io::Result<()>>)
    -> Result<Vec<Node>, Error>
{
    let mut nodes: Vec<Node> = Vec::new();
    inner( stream, headers, &mut nodes, always_use_files, f)?;
    Ok(nodes)
}

fn inner<R: BufRead>(
    reader: &mut R,
    headers: &http::HeaderMap,
    nodes: &mut Vec<Node>,
    always_use_files: bool,
    f: Option<fn(name: &mut FilePart) -> std::io::Result<()>>)
    -> Result<(), Error>
{
    let mut buf: Vec<u8> = Vec::new();

    let boundary = get_multipart_boundary(headers)?;

    // Read past the initial boundary
    let (_, found) = reader.stream_until_token(&boundary, &mut buf)?;
    if !found { return Err(Error::EofBeforeFirstBoundary); }

    // Define the boundary, including the line terminator preceding it.
    // Use their first line terminator to determine whether to use CRLF or LF.
    let (lt, ltlt, lt_boundary) = {
        let peeker = reader.fill_buf()?;
        if peeker.len() > 1 && &peeker[..2] == b"\r\n" {
            let mut output = Vec::with_capacity(2 + boundary.len());
            output.push(b'\r');
            output.push(b'\n');
            output.extend(boundary.clone());
            (vec![b'\r', b'\n'], vec![b'\r', b'\n', b'\r', b'\n'], output)
        } else if peeker.len() > 0 && peeker[0] == b'\n' {
            let mut output = Vec::with_capacity(1 + boundary.len());
            output.push(b'\n');
            output.extend(boundary.clone());
            (vec![b'\n'], vec![b'\n', b'\n'], output)
        } else {
            return Err(Error::NoCrLfAfterBoundary);
        }
    };

    loop {
        // If the next two lookahead characters are '--', parsing is finished.
        {
            let peeker = reader.fill_buf()?;
            if peeker.len() >= 2 && &peeker[..2] == b"--" {
                return Ok(());
            }
        }

        // Read the line terminator after the boundary
        let (_, found) = reader.stream_until_token(&lt, &mut buf)?;
        if !found { return Err(Error::NoCrLfAfterBoundary); }

        // Read the headers (which end in 2 line terminators)
        buf.truncate(0); // start fresh
        let (_, found) = reader.stream_until_token(&ltlt, &mut buf)?;
        if !found { return Err(Error::EofInPartHeaders); }

        // Keep the 2 line terminators as httparse will expect it
        buf.extend(ltlt.iter().cloned());

        // Parse the headers
        let mut part_headers = {
            let mut header_memory = [httparse::EMPTY_HEADER; 4];
            match httparse::parse_headers(&buf, &mut header_memory) {
                Ok(httparse::Status::Complete((_, raw_headers))) => {
                    let mut h = http::HeaderMap::new();
                    for x in raw_headers {
                        match header_name!(x.name){
                            Ok(header_name) => {
                                h.insert(header_name, header_value!(&String::from_utf8(x.value.to_vec()).unwrap_or_default()));
                            }
                            Err(_) => {}
                        }
                    }
                    Ok(h)
                }
                Ok(httparse::Status::Partial) => Err(Error::PartialHeaders),
                Err(err) => Err(From::from(err)),
            }?
        };

        // Check for a nested multipart
        let nested = {
            let ct = part_headers.get_mut(http::header::CONTENT_TYPE);
            if let Some(ct) = ct {
                let Mime(ref top_level, _, _) = {
                    match Mime::from_str(ct.to_str().unwrap_or_default()) {
                        Ok(v) => { Ok(v) }
                        Err(e) => {
                            Err(crate::Error::Parse("MimeParse error".to_string()))
                        }
                    }
                }?;
                *top_level == TopLevel::Multipart
            } else {
                false
            }
        };
        if nested {
            // Recurse:
            let mut inner_nodes: Vec<Node> = Vec::new();
            inner(reader, &part_headers, &mut inner_nodes, always_use_files, f)?;
            nodes.push(Node::Multipart((part_headers, inner_nodes)));
            continue;
        }

        let is_file = always_use_files || {
            let cd = part_headers.get(http::header::CONTENT_DISPOSITION);
            if cd.is_some() {
                let v = cd.unwrap().to_str().unwrap_or_default();
                if v.contains("attachment") {
                    true
                } else {
                    let mut r = false;
                    for x in v.split(";") {
                        if x.trim().starts_with("filename=") {
                            r = true;
                            break;
                        }
                    }
                    r
                }
            } else {
                false
            }
        };
        if is_file {
            // Setup a file to capture the contents.
            let mut filepart = FilePart::create(part_headers)?;

            match f {
                None => {}
                Some(f) => {
                    f(&mut filepart)?;
                    if let Some(w) = &mut filepart.write {
                        // Stream out the file.
                        let (read, found) = reader.stream_until_token(&lt_boundary, w)?;
                        if !found { return Err(Error::EofInFile); }
                        filepart.size = Some(read);
                        nodes.push(Node::File(filepart));
                    }
                }
            }
            // TODO: Handle Content-Transfer-Encoding.  RFC 7578 section 4.7 deprecated
            // this, and the authors state "Currently, no deployed implementations that
            // send such bodies have been discovered", so this is very low priority.
        } else {
            buf.truncate(0); // start fresh
            let (_, found) = reader.stream_until_token(&lt_boundary, &mut buf)?;
            if !found { return Err(Error::EofInPart); }

            nodes.push(Node::Part(Part {
                headers: part_headers,
                body: buf.clone(),
            }));
        }
    }
}

/// Get the `multipart/*` boundary string from `http::HeaderMap`
pub fn get_multipart_boundary(headers: &http::HeaderMap) -> Result<Vec<u8>, Error> {
    // Verify that the request is 'Content-Type: multipart/*'.
    let ct = match headers.get(http::header::CONTENT_TYPE) {
        Some(ct) => ct,
        None => return Err(Error::NoRequestContentType),
    };
    let mime = match Mime::from_str(ct.to_str().unwrap_or_default()) {
        Ok(v) => { Ok(v) }
        Err(_) => {
            Err(crate::Error::Parse("MimeParse Error".to_string()))
        }
    }?;
    let Mime(ref top_level, _, ref params) = mime;

    if *top_level != TopLevel::Multipart {
        return Err(Error::NotMultipart);
    }

    for &(ref attr, ref val) in params.iter() {
        if let (&Attr::Boundary, &Value::Ext(ref val)) = (attr, val) {
            let mut boundary = Vec::with_capacity(2 + val.len());
            boundary.extend(b"--".iter().cloned());
            boundary.extend(val.as_bytes());
            return Ok(boundary);
        }
    }
    Err(Error::BoundaryNotSpecified)
}

#[inline]
fn get_content_disposition_filename(cd: Option<&http::HeaderValue>) -> Result<Option<String>, Error> {
    match cd {
        None => {
            Ok(None)
        }
        Some(v) => {
            let vec: Vec<&str> = v.to_str().unwrap_or_default().split(";").collect();
            let mut idx = 0;
            for x in &vec {
                let x = x.trim();
                if x.starts_with("filename="){
                    return Ok(Some(x.trim_start_matches("filename=\"").trim_end_matches("\"").to_string()));
                }
                idx += 1;
            }
            return Ok(None);
        }
    }
    // if let Some(&DispositionParam::Filename(ref charset, _, ref bytes)) =
    // cd.parameters.iter().find(|&x| match *x {
    //     DispositionParam::Filename(_, _, _) => true,
    //     _ => false,
    // })
    // {
    //     match charset_decode(charset, bytes) {
    //         Ok(filename) => Ok(Some(filename)),
    //         Err(e) => Err(Error::Decoding(e)),
    //     }
    // } else {
    //     Ok(None)
    // }
}

// This decodes bytes encoded according to a hyper::header::Charset encoding, using the
// rust-encoding crate.  Only supports encodings defined in both crates.
fn charset_decode(charset: &Charset, bytes: &[u8]) -> Result<String, Cow<'static, str>> {
    Ok(match *charset {
        Charset::Us_Ascii => all::ASCII.decode(bytes, DecoderTrap::Strict)?,
        Charset::Iso_8859_1 => all::ISO_8859_1.decode(bytes, DecoderTrap::Strict)?,
        Charset::Iso_8859_2 => all::ISO_8859_2.decode(bytes, DecoderTrap::Strict)?,
        Charset::Iso_8859_3 => all::ISO_8859_3.decode(bytes, DecoderTrap::Strict)?,
        Charset::Iso_8859_4 => all::ISO_8859_4.decode(bytes, DecoderTrap::Strict)?,
        Charset::Iso_8859_5 => all::ISO_8859_5.decode(bytes, DecoderTrap::Strict)?,
        Charset::Iso_8859_6 => all::ISO_8859_6.decode(bytes, DecoderTrap::Strict)?,
        Charset::Iso_8859_7 => all::ISO_8859_7.decode(bytes, DecoderTrap::Strict)?,
        Charset::Iso_8859_8 => all::ISO_8859_8.decode(bytes, DecoderTrap::Strict)?,
        Charset::Iso_8859_9 => return Err("ISO_8859_9 is not supported".into()),
        Charset::Iso_8859_10 => all::ISO_8859_10.decode(bytes, DecoderTrap::Strict)?,
        Charset::Shift_Jis => return Err("Shift_Jis is not supported".into()),
        Charset::Euc_Jp => all::EUC_JP.decode(bytes, DecoderTrap::Strict)?,
        Charset::Iso_2022_Kr => return Err("Iso_2022_Kr is not supported".into()),
        Charset::Euc_Kr => return Err("Euc_Kr is not supported".into()),
        Charset::Iso_2022_Jp => all::ISO_2022_JP.decode(bytes, DecoderTrap::Strict)?,
        Charset::Iso_2022_Jp_2 => return Err("Iso_2022_Jp_2 is not supported".into()),
        Charset::Iso_8859_6_E => return Err("Iso_8859_6_E is not supported".into()),
        Charset::Iso_8859_6_I => return Err("Iso_8859_6_I is not supported".into()),
        Charset::Iso_8859_8_E => return Err("Iso_8859_8_E is not supported".into()),
        Charset::Iso_8859_8_I => return Err("Iso_8859_8_I is not supported".into()),
        Charset::Gb2312 => return Err("Gb2312 is not supported".into()),
        Charset::Big5 => all::BIG5_2003.decode(bytes, DecoderTrap::Strict)?,
        Charset::Koi8_R => all::KOI8_R.decode(bytes, DecoderTrap::Strict)?,
        Charset::Ext(ref s) => match &**s {
            "UTF-8" => all::UTF_8.decode(bytes, DecoderTrap::Strict)?,
            _ => return Err("Encoding is not supported".into()),
        },
    })
}

/// Generate a valid multipart boundary, statistically unlikely to be found within
/// the content of the parts.
pub fn generate_boundary() -> Vec<u8> {
    TextNonce::sized(68).unwrap().into_string().into_bytes()
}

// Convenience method, like write_all(), but returns the count of bytes written.
trait WriteAllCount {
    fn write_all_count(&mut self, buf: &[u8]) -> ::std::io::Result<usize>;
}

impl<T: Write> WriteAllCount for T {
    fn write_all_count(&mut self, buf: &[u8]) -> ::std::io::Result<usize>
    {
        self.write_all(buf)?;
        Ok(buf.len())
    }
}

/// Stream a multipart body to the output `stream` given, made up of the `parts`
/// given.  Top-level headers are NOT included in this stream; the caller must send
/// those prior to calling write_multipart().
/// Returns the number of bytes written, or an error.
pub fn write_multipart<S: Write>(
    stream: &mut S,
    boundary: &Vec<u8>,
    nodes: &mut Vec<Node>,
    file: Option<fn(name: &mut FilePart) -> std::io::Result<()>>)
    -> Result<usize, Error>
{
    let mut count: usize = 0;

    for node in nodes {
        // write a boundary
        count += stream.write_all_count(b"--")?;
        count += stream.write_all_count(&boundary)?;
        count += stream.write_all_count(b"\r\n")?;

        match node {
            &mut Node::Part(ref part) => {
                // write the part's headers
                for (name, value) in part.headers.iter() {
                    count += stream.write_all_count(name.as_str().as_bytes())?;
                    count += stream.write_all_count(b": ")?;
                    count += stream.write_all_count(value.as_bytes())?;
                    count += stream.write_all_count(b"\r\n")?;
                }

                // write the blank line
                count += stream.write_all_count(b"\r\n")?;

                // Write the part's content
                count += stream.write_all_count(&part.body)?;
            }
            &mut Node::File(ref mut filepart) => {
                // write the part's headers
                for (name, value) in filepart.headers.iter() {
                    count += stream.write_all_count(name.as_str().as_bytes())?;
                    count += stream.write_all_count(b": ")?;
                    count += stream.write_all_count(value.as_bytes())?;
                    count += stream.write_all_count(b"\r\n")?;
                }

                // write the blank line
                count += stream.write_all_count(b"\r\n")?;

                // Write out the files's content
                match file {
                    None => {}
                    Some(f) => {
                        f(filepart)?;
                        count += std::io::copy(filepart.write.as_mut().unwrap(), stream)? as usize;
                    }
                }
            }
            &mut Node::Multipart((ref headers, ref mut subnodes)) => {
                // Get boundary
                let boundary = get_multipart_boundary(headers)?;

                // write the multipart headers
                for (name, value) in headers.iter() {
                    count += stream.write_all_count(name.as_str().as_bytes())?;
                    count += stream.write_all_count(b": ")?;
                    count += stream.write_all_count(value.as_bytes())?;
                    count += stream.write_all_count(b"\r\n")?;
                }

                // write the blank line
                count += stream.write_all_count(b"\r\n")?;

                // Recurse
                count += write_multipart(stream, &boundary, subnodes, file)?;
            }
        }

        // write a line terminator
        count += stream.write_all_count(b"\r\n")?;
    }

    // write a final boundary
    count += stream.write_all_count(b"--")?;
    count += stream.write_all_count(&boundary)?;
    count += stream.write_all_count(b"--")?;

    Ok(count)
}

pub fn write_chunk<S: Write>(
    stream: &mut S,
    chunk: &[u8]) -> Result<(), ::std::io::Error>
{
    write!(stream, "{:x}\r\n", chunk.len())?;
    stream.write_all(chunk)?;
    stream.write_all(b"\r\n")?;
    Ok(())
}

/// Stream a multipart body to the output `stream` given, made up of the `parts`
/// given, using Tranfer-Encoding: Chunked.  Top-level headers are NOT included in this
/// stream; the caller must send those prior to calling write_multipart_chunked().
pub fn write_multipart_chunked<S: Write>(
    stream: &mut S,
    boundary: &Vec<u8>,
    nodes: &Vec<Node>)
    -> Result<(), Error>
{
    for node in nodes {
        // write a boundary
        write_chunk(stream, b"--")?;
        write_chunk(stream, &boundary)?;
        write_chunk(stream, b"\r\n")?;

        match node {
            &Node::Part(ref part) => {
                // write the part's headers
                for (name, value) in part.headers.iter() {
                    write_chunk(stream, name.as_str().as_bytes())?;
                    write_chunk(stream, b": ")?;
                    write_chunk(stream, value.as_bytes())?;
                    write_chunk(stream, b"\r\n")?;
                }

                // write the blank line
                write_chunk(stream, b"\r\n")?;

                // Write the part's content
                write_chunk(stream, &part.body)?;
            }
            &Node::File(ref filepart) => {
                // write the part's headers
                for (name, value) in filepart.headers.iter() {
                    write_chunk(stream, name.as_str().as_bytes())?;
                    write_chunk(stream, b": ")?;
                    write_chunk(stream, value.as_bytes())?;
                    write_chunk(stream, b"\r\n")?;
                }

                // write the blank line
                write_chunk(stream, b"\r\n")?;

                // Write out the files's length
                let metadata = std::fs::metadata(&filepart.path)?;
                write!(stream, "{:x}\r\n", metadata.len())?;

                // Write out the file's content
                let mut file = File::open(&filepart.path)?;
                std::io::copy(&mut file, stream)? as usize;
                stream.write(b"\r\n")?;
            }
            &Node::Multipart((ref headers, ref subnodes)) => {
                // Get boundary
                let boundary = get_multipart_boundary(headers)?;

                // write the multipart headers
                for (k, v) in headers.iter() {
                    write_chunk(stream, k.as_str().as_bytes())?;
                    write_chunk(stream, b": ")?;
                    write_chunk(stream, v.as_bytes())?;
                    write_chunk(stream, b"\r\n")?;
                }

                // write the blank line
                write_chunk(stream, b"\r\n")?;

                // Recurse
                write_multipart_chunked(stream, &boundary, &subnodes)?;
            }
        }

        // write a line terminator
        write_chunk(stream, b"\r\n")?;
    }

    // write a final boundary
    write_chunk(stream, b"--")?;
    write_chunk(stream, &boundary)?;
    write_chunk(stream, b"--")?;

    // Write an empty chunk to signal the end of the body
    write_chunk(stream, b"")?;

    Ok(())
}
