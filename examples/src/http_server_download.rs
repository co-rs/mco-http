#[deny(unused_variables)]
extern crate mco_http;
extern crate fast_log;
extern crate serde_json;

use fast_log::config::Config;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Deref;

use std::path::PathBuf;
use std::sync::mpsc::RecvError;
use captcha::Captcha;
use captcha::filters::{Dots, Noise, Wave};
use mco::{chan, defer, spawn_blocking};
use mco::std::lazy::sync::Lazy;
use mco::std::sync::{Receiver, Sender};
use mco_http::header::ContentType;
use mco_http::server::{Request, Response};

/// Draw a captcha code and display it on the web
fn download(mut req: Request, res: Response) {
    //first set header content type
    res.headers.set(ContentType::png());
    //next,req thread new an png to me
    //Heavy computing tasks should be performed by threads
    let png = spawn_blocking!(||{
        let mut captcha = Captcha::new();
        captcha
            .add_chars(4)
            .apply_filter(Noise::new(0.1))
            .apply_filter(Wave::new(1.0, 10.0).horizontal())
            // .apply_filter(Wave::new(2.0, 20.0).vertical())
            .view(160, 60)
            .apply_filter(Dots::new(4));
        return captcha.as_png().unwrap_or_default() as Vec<u8>;
    });
    //return data
    res.send(&png.unwrap_or_default()).unwrap();
}

fn main() {
    let _ = fast_log::init(Config::new().level(log::LevelFilter::Info).console());

    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(download).unwrap();
    println!("Listening on http://127.0.0.1:3000");
}
