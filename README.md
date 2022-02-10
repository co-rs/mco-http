# cogo-http

* Coroutine HTTP framework for Cogo, Original code fork from Hyper，We improved the underlying logic
* support http server
* support http client
## Performance

* platform(16CPU/32 thread AMD Ryzen 9 5950X,32GB mem,Os:Unbutu-20.04)
* [TechEmpowerBench fork project](https://github.com/zhuxiujia/FrameworkBenchmarks)

![per](docs/629a066aaa37b4c295fa794c5ebdf31.png)

## example-server
```rust
#[deny(unused_variables)]
extern crate cogo_http;

use cogo_http::route::Route;
use cogo_http::server::{Request, Response};

fn hello(req: Request, res: Response) {
    res.send(b"Hello World!").unwrap();
}

fn main() {
    let _listening = cogo_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(hello);
    println!("Listening on http://127.0.0.1:3000");
}

```

## example-client
```rust
extern crate cogo_http;

use std::io;
use cogo_http::Client;
use cogo_http::header::Connection;

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
