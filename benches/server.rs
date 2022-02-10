#![deny(warnings)]
#![feature(test)]
extern crate cogo_http;
extern crate test;


use cogo_http::mock::MockStream;
use cogo_http::server::{Request, Response, Worker};

#[bench]
fn bench_mock_handle_connection(b: &mut test::Bencher) {
    let mut mock = MockStream::with_input(b"\
            POST /upload HTTP/1.1\r\n\
            Host: example.domain\r\n\
            Expect: 100-continue\r\n\
            Content-Length: 10\r\n\
            \r\n\
            1234567890\
        ");

    fn handle(_: Request, res: Response) {
        res.start().unwrap().end().unwrap();
    }
    let w=Worker::new(handle, Default::default());
    b.iter(|| {
        w.handle_connection(&mut mock);
    });
}