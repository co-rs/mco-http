use std::sync::Arc;
use std::time::Duration;
use mco_http::route::Route;
use mco_http::server::{Request, Response};
use mco_http::route::MiddleWare;

use serde::{Serialize, Deserialize};
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use jsonwebtoken::errors::ErrorKind;
use mco::std::time::Time;
use mco_http::query::read_query;
use mco_http::uri::RequestUri;

/// JWT 鉴权 Token结构
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct JWTToken {
    //account id
    pub id: String,
    //account
    pub account: String,
    //permissions
    pub permissions: Vec<String>,
    //role_ids
    pub role_ids: Vec<String>,
    //Expiration time
    pub exp: usize,
}

impl JWTToken {
    /// create token
    /// secret: your secret string
    pub fn create_token(&self, secret: &str) -> Result<String, String> {
        return match encode(
            &Header::default(),
            self,
            &EncodingKey::from_secret(secret.as_ref()),
        ) {
            Ok(t) => Ok(t),
            Err(_) => Err(String::from("JWTToken encode fail!")), // in practice you would return the error
        };
    }
    /// verify token invalid
    /// secret: your secret string
    pub fn verify(secret: &str, token: &str) -> Result<JWTToken, String> {
        let validation = Validation::new(Algorithm::HS256);
        return match decode::<JWTToken>(
            &token,
            &DecodingKey::from_secret(secret.as_ref()),
            &validation,
        ) {
            Ok(c) => Ok(c.claims),
            Err(err) => match *err.kind() {
                ErrorKind::InvalidToken => return Err(String::from("InvalidToken")), // Example on how to handle a specific error
                ErrorKind::InvalidIssuer => return Err(String::from("InvalidIssuer")), // Example on how to handle a specific error
                _ => return Err(String::from("InvalidToken other errors")),
            },
        };
    }
}

// MiddleWare
#[derive(Debug)]
pub struct MyMiddleWare {
    secret: String,
}

impl MiddleWare for MyMiddleWare {
    fn handle(&self, req: &mut Request, resp: &mut Option<Response>) {
        //not check login api
        match &req.uri {
            RequestUri::AbsolutePath(p) => {
                if p.eq("/login") {
                    return;
                }
            }
            RequestUri::AbsoluteUri(_) => {}
            RequestUri::Authority(_) => {}
            RequestUri::Star => {}
        }
        let querys = read_query(&req.uri.to_string());
        let token_value = querys.get("token");
        if token_value.is_none() {
            resp.take().unwrap().send("token must be in path.for example: http://127.0.0.1:3000?token=xxxxx".as_bytes());
            return;
        }
        let token = token_value.unwrap();
        if token.is_empty() {
            resp.take().unwrap().send("token must be in path.for example: http://127.0.0.1:3000?token=xxxxx".as_bytes());
            return;
        }
        match JWTToken::verify(&self.secret, token) {
            Ok(v) => {
                req.extra.insert(v);
            }
            Err(e) => {
                resp.take().unwrap().send(format!("Token is invalid! {}", e).as_bytes());
            }
        }
    }
}


fn main() {
    env_logger::init().unwrap();

    let mut route = Route::new();
    let middle = Arc::new(MyMiddleWare { secret: "123456".to_string() });
    route.add_middleware(middle.clone());
    route.handle_fn("/", |req: Request, mut res: Response| {
        let login_user_data = req.extra.get::<JWTToken>().unwrap();
        res.send(format!("read from middleware: {:?}", login_user_data).as_bytes());
    });
    let login_fn = move || -> String {
        let jwt = JWTToken {
            id: "111".to_string(),
            account: "222".to_string(),
            permissions: vec![],
            role_ids: vec![],
            exp: {
                // timeout for 24 hour
                let mut now = Time::now();
                now.add(Duration::from_secs(24 * 3600));
                now.unix_timestamp() as usize
            },
        };
        let token = jwt.create_token(middle.secret.as_str()).unwrap();
        return token;
    };
    let token = login_fn();
    route.handle_fn("/login", move |req: Request, mut res: Response| {
        res.send(format!("{}", login_fn()).as_bytes());
    });

    let route = Arc::new(route);
    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(route.clone());
    println!("Listening on http://127.0.0.1:3000/login");
    println!("then click http://127.0.0.1:3000?token={}", token);
}