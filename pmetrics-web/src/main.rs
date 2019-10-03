extern crate serde_json;
extern crate iron;
extern crate router;
extern crate postgres;
//extern crate r2d2_postgres;
extern crate chrono;

use iron::prelude::*;
use iron::status;
use std::io::Read;
use router::Router;
use serde_json::{Value, Error};
use std::thread;
use std::env;
//use r2d2_postgres::PostgresConnectionManager;
use postgres::{Connection, TlsMode};
use chrono::prelude::{DateTime, Utc};
use std::collections::HashMap;


struct Measure {
    name: String,
    insertion_time: DateTime<Utc>,
    measurement: f64,
    dict: Value
}

struct Log {
    insertion_time: DateTime<Utc>,
    log:  String,
}

#[derive(Debug)]
struct Event {
    insertion_time: DateTime<Utc>,
    name: String,
    dict: Value
}

fn postevent(req: &mut Request) -> IronResult<Response> {
    let mut buffer = String::new();
    req.body.read_to_string(&mut buffer);
    println!("Event: {}", buffer);
    serde_json::from_str(&buffer).map(|v:Value|
                                      println!("Event Structured: {}", v));
    Ok(Response::with((status::Ok, "ok")))
}
fn getevent(req: &mut Request) -> IronResult<Response> {
    let mut buffer = String::new();
    req.body.read_to_string(&mut buffer);
    println!("Event: {}", buffer);
    serde_json::from_str(&buffer).map(|v:Value|
                                      println!("Event Structured: {}", v));
    Ok(Response::with((status::Ok, "ok")))
}
fn handler(req: &mut Request) -> IronResult<Response> {
        let ref query = req.extensions.get::<Router>().unwrap().find("query").unwrap_or("/");
        Ok(Response::with((status::Ok, *query)))
}
fn main() {
    let envmap: HashMap<String, String> =  env::vars().into_iter().collect();
    let pguser = match envmap.get("PGUSER") {
        Some(s) => s,
        None => ""
    };
    let pgdb = match envmap.get("PGPASS") {
        Some(s) =>  s,
        None => ""
    };

    let pgport = match envmap.get("PGPORT") {
        Some(s) => s.parse::<i32>().unwrap(),
        None => 5432
    };
    let pghost = match envmap.get("PGHOST") {
        Some(s) => s,
        None => "localhost"
    };
    let pgdb = match envmap.get("PGDATABASE") {
        Some(s) => s,
        None => "postgres"
    };


    /*for (key, value) in hm  {
        println!("{}: {}", key, value);
    }*/
    /*
actually we're going to break this code up:

- single-shot notifications for


     */
    let conn = Connection::connect("postgres://postgres:aargh@localhost:5432", TlsMode::None).unwrap();


    for row in &conn.query("SELECT insertion_time, name, dict from monitoring.event", &[]).unwrap() {
        let event = Event{
            insertion_time: row.get(0),
            name: row.get(1),
            dict: row.get(2)
        };
        println!("event: {:?}", event);
    }


    /*
maek this workz
let manager = PostgresConnectionManager::new(
        "host=localhost user=postgres password=aaargh port=5432 database=postgres".parse().unwrap(),
        NoTls,
    );
    let pool = r2d2_postgres::Pool::new(manager).unwrap();
*/

    let mut router = Router::new();           // Alternative syntax:
    router.get("/", handler, "index");        // let router = router!(index: get "/" => handler,
    router.post("/api/v1/event", postevent, "event");
    router.get("/api/v1/event", getevent, "event");

    Iron::new(router).http("localhost:3000").unwrap();

}
