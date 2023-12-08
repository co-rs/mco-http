#[deny(unused_variables)]
extern crate mco_http;
extern crate fast_log;

use fast_log::config::Config;
use tungstenite::{accept_hdr, handshake};

fn main() {
    let _=fast_log::init(Config::new().console());
    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle_accept(|stream| {
            let callback = |req: &handshake::server::Request, response: handshake::server::Response| {
                println!("Received a new ws handshake");
                println!("The request's path is: {}", req.uri().to_string());
                println!("The request's headers are:");
                Ok(response)
            };
            let mut websocket = accept_hdr(stream, callback).unwrap();
            println!("start reading websocket....................");
            loop {
                let msg = websocket.read().unwrap();
                if msg.is_binary() || msg.is_text() {
                    println!("read msg={}",msg);
                    websocket.send(msg).unwrap();
                }
            }
        });
    println!("Listening on http://127.0.0.1:3000");
}
