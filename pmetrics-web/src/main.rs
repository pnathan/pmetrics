extern crate serde_json;
extern crate iron;
extern crate router;
extern crate postgres;

use iron::prelude::*;
use iron::status;
use std::io::Read;
use router::Router;
use serde_json::{Value, Error};

use postgres::{Connection, TlsMode};


struct Measure {
    name: String,
    measurement: f64,
    dict: Value
}

struct Log {
    log:  String,
}


fn event(req: &mut Request) -> IronResult<Response> {
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

    let mut router = Router::new();           // Alternative syntax:
    router.get("/", handler, "index");        // let router = router!(index: get "/" => handler,
    router.post("/api/v1/event", event, "event");  //                      query: get "/:query" => handler);

    Iron::new(router).http("localhost:3000").unwrap();

}
