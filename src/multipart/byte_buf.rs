use std::io::{Read, Write};

pub struct ByteBuffer {
    pub inner: Vec<u8>,
}

impl Write for ByteBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for x in buf {
            self.inner.push(x.clone());
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Read for ByteBuffer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut idx = 0;
        for x in &self.inner {
            if idx < buf.len() {
                buf[idx] = x.clone();
            } else {
                break;
            }
            idx += 1;
        }
        Ok(idx)
    }
}