#[deny(unused_variables)]
extern crate mco_http;
extern crate fast_log;

use fast_log::config::Config;
use tungstenite::{accept_hdr, handshake};
use mco_http::server::{Request, Response};



fn main() {
    let _=fast_log::init(Config::new().console());
    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(|mut _req: Request, res: Response| {
            let callback = |req: &handshake::server::Request, mut response: handshake::server::Response| {
                println!("Received a new ws handshake");
                println!("The request's path is: {}", req.uri().to_string());
                println!("The request's headers are:");
                // Let's add an additional header to our response to the client.
                // let headers = response.headers_mut();
                Ok(response)
            };
            let stream = _req.body.get_mut().get_mut();
            let mut websocket = accept_hdr(stream, callback).unwrap();
            loop {
                let msg = websocket.read().unwrap();
                if msg.is_binary() || msg.is_text() {
                    websocket.send(msg).unwrap();
                }
            }
        });
    println!("Listening on http://127.0.0.1:3000");
}
