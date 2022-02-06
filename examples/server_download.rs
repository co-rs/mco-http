#[deny(unused_variables)]
extern crate cogo_http;
extern crate env_logger;
extern crate serde_json;

use std::fs::File;
use std::io::{Read, Write};
use std::ops::Deref;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::RecvError;
use captcha::Captcha;
use captcha::filters::{Dots, Noise, Wave};
use cogo::{chan, defer};
use cogo::std::lazy::sync::Lazy;
use cogo::std::sync::{Receiver, Sender};
use cogo_http::header::ContentType;
use cogo_http::server::{Request, Response};

/// Draw a captcha code and display it on the web
fn download(mut req: Request, res: Response) {
    //first set header content type
    res.headers.set(ContentType::png());


    //next,req thread new an png to me
    let c = Arc::new(chan!(1));
    //Heavy computing tasks should be performed by threads
    let channel_clone = c.clone();
    std::thread::spawn(move || {
        match channel_clone.1.recv() {
            Ok(v) => {
                defer!(||{
                    // make sure that messages will be sent even if panic occurs
                    if channel_clone.0.remain() == 0 {
                        channel_clone.0.send(vec![]);
                    }
                });
                let mut captcha = Captcha::new();
                captcha
                    .add_chars(4)
                    .apply_filter(Noise::new(0.1))
                    .apply_filter(Wave::new(1.0, 10.0).horizontal())
                    // .apply_filter(Wave::new(2.0, 20.0).vertical())
                    .view(160, 60)
                    .apply_filter(Dots::new(4));
                let png = captcha.as_png().unwrap();
                channel_clone.0.send(png);
            }
            Err(_) => {}
        }
    });
    c.0.send(vec![]);
    let png = c.1.recv().unwrap_or_default();


    //return data
    res.send(&png).unwrap();
}

fn main() {
    env_logger::init().unwrap();

    let _listening = cogo_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(download).unwrap();
    println!("Listening on http://127.0.0.1:3000");
}
