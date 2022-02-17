# mco-http

* HTTP framework based on [mco](https://github.com/co-rs/mco),  Original code fork from Hyper，We improved the underlying logic
* support http server
* support http client
* support route
* support Interceptor/middleware
## Performance

* platform(16CPU/32 thread AMD Ryzen 9 5950X,32GB mem,Os:Unbutu-20.04)
* [TechEmpowerBench fork project](https://github.com/zhuxiujia/FrameworkBenchmarks)

![per](docs/629a066aaa37b4c295fa794c5ebdf31.png)

## example-server
```rust
#[deny(unused_variables)]
extern crate mco_http;

use mco_http::route::Route;
use mco_http::server::{Request, Response};

fn hello(req: Request, res: Response) {
    res.send(b"Hello World!");
}

fn main() {
    let mut route = Route::new();
    route.handle_fn("/", |req: Request, res: Response| {
        res.send(b"Hello World!");
    });
    route.handle_fn("/js", |req: Request, res: Response| {
        res.send("{\"name\":\"joe\"}".as_bytes());
    });
    route.handle_fn("/fn", hello);
    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello);
    println!("Listening on http://127.0.0.1:3000");
}

```

## example-client
```rust
extern crate mco_http;

use std::io;
use mco_http::Client;
use mco_http::header::Connection;

fn main() {
    let mut url = "http://www.baidu.com".to_string();

    let client = Client::new();

    let mut res = client.get(&url)
        .header(Connection::close())
        .send().unwrap();

    println!("Response: {}", res.status);
    println!("Headers:\n{}", res.headers);
    io::copy(&mut res, &mut io::stdout()).unwrap();
}
```
