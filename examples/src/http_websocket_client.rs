extern crate mco_http;
extern crate fast_log;

use std::time::Duration;
use tungstenite::{connect, Message};
use tungstenite::protocol::Role;
use tungstenite::stream::MaybeTlsStream;

fn main() {
    let (mut s, response) =
        connect(url::Url::parse("ws://127.0.0.1:3000").unwrap()).expect("Can't connect");

    //replace TcpStream
    let v = s.get_mut();
    let stream = match v {
        MaybeTlsStream::Plain(v) => {
            v.try_clone().expect("clone stream fail")
        }
        _ => {
            panic!("unknown stream")
        }
    };
    let mco_tcp = mco::net::TcpStream::new(stream).unwrap();
    let mut socket = tungstenite::WebSocket::from_raw_socket(mco_tcp, Role::Client, Option::from(s.get_config().clone()));
    socket.send(Message::Text("Hello mco WebSocket".to_string())).unwrap();
    let msg = socket.read().expect("Error reading message");
    mco::coroutine::sleep(Duration::from_secs(1));
    println!("Received: {}", msg);
}
