extern crate mco_http;
extern crate fast_log;

use fast_log::config::Config;
use tungstenite::{connect, Message};

//FIXME: use mco tcp stream
fn main() {
    let _ = fast_log::init(Config::new().level(log::LevelFilter::Info).console());

    let (mut socket, response) =
        connect(url::Url::parse("ws://127.0.0.1:3000").unwrap()).expect("Can't connect");
    socket.send(Message::Text("Hello WebSocket".to_string())).unwrap();
    loop {
        let msg = socket.read().expect("Error reading message");
        println!("Received: {}", msg);
    }
}
