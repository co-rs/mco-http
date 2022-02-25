#[deny(unused_variables)]
extern crate mco_http;
extern crate env_logger;

#[macro_use]
extern crate cdbc;

use std::fs::File;
use std::ops::Deref;
use std::sync::Arc;
use cdbc::Executor;
use cdbc_sqlite::SqlitePool;
use mco_http::route::Route;
use mco_http::server::{Request, Response};
use cdbc::scan::Scan;

/// table
#[derive(Debug, serde::Serialize, serde::Deserialize, cdbc::Scan)]
pub struct BizActivity {
    pub id: Option<String>,
    pub name: Option<String>,
    pub delete_flag: Option<i32>,
}

impl BizActivity {
    pub fn find_all(pool: &SqlitePool) -> cdbc::Result<Vec<Self>> {
        let data =
            cdbc::query("select * from biz_activity limit 1")
                .fetch_all(pool)
                .scan()?;
        Ok(data)
    }

    pub fn count(pool: &SqlitePool) -> cdbc::Result<i32> {
        #[derive(Debug,cdbc::Scan)]
        pub struct Count {
            pub count: Option<i32>,
        }
        let row: Count = cdbc::query("select count(1) as count from biz_activity limit 1")
            .fetch_one(pool).scan()?;
        Ok(row.count.unwrap_or_default())
    }
}

pub trait Controllers {
    fn get_pool(&self) -> &SqlitePool;
    fn find_all(&self, req: Request, res: Response);
}

impl Controllers for Route {
    fn get_pool(&self) -> &SqlitePool {
        self.index::<SqlitePool>("sqlite")
    }

    fn find_all(&self, req: Request, res: Response) {
        let records = BizActivity::find_all(self.get_pool()).unwrap();
        res.send(serde_json::json!(records).to_string().as_bytes());
    }
}


fn main() {
    env_logger::init().unwrap();

    let mut route = Arc::new(Route::new());
    //insert pool
    route.insert("sqlite", make_sqlite().unwrap());

    let route_clone = route.clone();
    route.handle_fn("/", move |req: Request, res: Response| {
        route_clone.find_all(req, res);
    });
    let _listening = mco_http::Server::http("0.0.0.0:3000").unwrap()
        .handle(route);
    println!("Listening on http://127.0.0.1:3000");
}

fn make_sqlite() -> cdbc::Result<SqlitePool> {
    //first. create sqlite dir/file
    std::fs::create_dir_all("target/db/");
    File::create("target/db/sqlite.db");
    //next create table and query result
    let pool = SqlitePool::connect("sqlite://target/db/sqlite.db")?;
    let mut conn = pool.acquire()?;
    conn.execute("CREATE TABLE biz_activity(  id string, name string,age int, delete_flag int) ");
    conn.execute("INSERT INTO biz_activity (id,name,age,delete_flag) values (\"1\",\"1\",1,0)");
    Ok(pool)
}

