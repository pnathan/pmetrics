extern crate chrono;
extern crate iron;
extern crate router;
extern crate serde;
extern crate serde_json;
extern crate postgres;

use std::collections::HashMap;
use std::env;
use std::io::Read;

use chrono::prelude::{DateTime, Utc};
use iron::prelude::{IronResult, Request, Response, Iron};
use iron::status;
use postgres::{Connection, TlsMode};
use router::Router;
use serde::{Deserialize, Serialize};
use serde_json::{Value};

//use r2d2_postgres::PostgresConnectionManager;

mod audit;

#[derive(Debug, Serialize, Deserialize)]
struct MeasureIngest {
    name: String,
    measurement: f64,
    dict: Value
}

#[derive(Debug, Serialize, Deserialize)]
struct LogIngest {
    log:  String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EventIngest {
    name: String,
    dict: Value
}

#[derive(Debug, Serialize, Deserialize)]
struct Measure {
    d: MeasureIngest,
    insertion_time: DateTime<Utc>
}

#[derive(Debug, Serialize, Deserialize)]
struct Log {
    d: LogIngest,
    insertion_time: DateTime<Utc>
}

#[derive(Debug, Serialize, Deserialize)]
struct Event {
    d: EventIngest,
    insertion_time: DateTime<Utc>
}


// Stab at a generic POST.
// Doesn't compile.
// Need to factor out the deserialization.
/*
fn poast<T: std::fmt::Debug + Deserialize> (req: &mut Request, auditor: audit::Audit) -> IronResult<Response> {
    auditor.tell(&audit::Concern::Debug(audit::Event::new("started", "http connection")));
    let mut buffer = String::new();
    req.body.read_to_string(&mut buffer);
    let deserialized: T = serde_json::from_str(&buffer).unwrap();
    //println!("{:?}", deserialized);
    auditor.tell(&audit::Concern::Debug(audit::Event::new("ending", "http connection")));
    Ok(Response::with((status::Ok, "ok")))
}
*/
fn postlog(req: &mut Request, auditor: &audit::Audit) -> IronResult<Response> {

    let conn = connect_to_db(&auditor);
    auditor.tell(&audit::Concern::Debug(audit::Event::new("started", "http connection")));
    let mut buffer = String::new();
    req.body.read_to_string(&mut buffer);
    let deserialized: LogIngest = serde_json::from_str(&buffer).unwrap();
    // TODO: proper match arms and http answers
    conn.execute("INSERT INTO monitoring.log (logtext) VALUES ($1)", &[&deserialized.log]).unwrap();
    auditor.debug(audit::Event::new("ending", "http connection"));
    Ok(Response::with((status::Ok, "ok")))
}

fn getlog(req: &mut Request, auditor: &audit::Audit) -> IronResult<Response> {
    let conn = connect_to_db(&auditor);
    auditor.debug(audit::Event::new("started", "http connection"));

    let mut vec: Vec<Log> = Vec::new();

    for row in &conn.query("SELECT insertion_time, logtext from monitoring.log order by insertion_time desc limit 100", &[]).unwrap() {
        let log = Log {
            insertion_time: row.get(0),
            d: LogIngest {
                log: row.get(1),
            }
        };

        vec.push(log);
    }

    let result = serde_json::to_string(&vec).unwrap();
    auditor.tell(&audit::Concern::Debug(audit::Event::new("ending", "http connection")));
    Ok(Response::with((status::Ok, result)))
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

struct PostgresClientArgs {
    user: String,
    password: String,
    host: String,
    port: u16,
    db: String,
}

impl PostgresClientArgs {
    fn from_env() -> PostgresClientArgs {
        let envmap: HashMap<String, String> =  env::vars().into_iter().collect();
        let pguser = match envmap.get("PGUSER") {
            Some(s) => s,
            None => ""
        };
        let pgpass = match envmap.get("PGPASSWORD") {
            Some(s) =>  s,
            None => ""
        };

        let pgport = match envmap.get("PGPORT") {
            Some(s) => s.parse::<u16>().unwrap(),
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

        PostgresClientArgs {
            user: pguser.to_string(),
            password: pgpass.to_string(),
            host: pghost.to_string(),
            port: pgport,
            db: pgdb.to_string()
        }
    }

    fn connection_string(&self) -> String {
        format!("postgres://{}:{}@{}:{}/{}", self.user, self.password, self.host, self.port, self.db)
    }
}

#[derive(Copy, Clone)]
struct GlobalOptions {
    verbosity: audit::ConcernLevel
}

enum ServerType {
    Http,
    NamedPipe,
    Grpc
}
struct ServerOptions {
    port: u16
}

enum Command {
    CliClient(GlobalOptions),
    Server(GlobalOptions, ServerOptions, ServerType),
    Debugger(GlobalOptions)
}

fn connect_to_db(auditor: &audit::Audit) -> postgres::Connection {
    let cs = PostgresClientArgs::from_env().connection_string();
    println!("CS: {}", &cs);
    auditor.tell(&audit::Concern::Info(audit::Event::new("starting", "pg conn")));
    let conn = Connection::connect(cs, TlsMode::None).unwrap();
    auditor.tell(&audit::Concern::Info(audit::Event::new("started", "pg conn")));

    conn
}

fn cliparser() -> Command {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];
    let go = GlobalOptions { verbosity: audit::ConcernLevel::Debug};
    match command.as_ref() {
        "cli" => {
            // reads from standard in.
            Command::CliClient(go)
        }
        "server" => {
            let servertype = &args[2];
            match servertype.as_ref() {
                "http" => {
                    Command::Server(go,
                                    ServerOptions { port: 1337 },
                                    ServerType::Http)
                }
                _ => {
                    panic!("I only speak http server, not {}", servertype);
                }
            }
        }
        "debug" => {
            Command::Debugger(go)
        }
        _ => {
            panic!("I don't understand the command: {}", command);
        }
    }

}

fn main() {
    /*
actually we're going to break this code up:

- single-shot dispatch

- http server

- potentially a named pipe reader?

- grpc server
     */

    let cmd = cliparser();

    let cl = match cmd {
        Command::Server(go, _, _) => go.verbosity,
        Command::Debugger(go) => go.verbosity,
        Command::CliClient(go) => go.verbosity
    };

    let auditor =  audit::Audit::new(cl);

    match cmd {
        Command::Server(_, server_options, servertype) => {
            match servertype {
                ServerType::Http => {

                    let mut router = Router::new();
                    router.get("/", handler, "index");

                    router.post("/api/v1/event", postevent, "event");
                    router.get("/api/v1/event", getevent, "event");


                    router.post("/api/v1/log", move |req: &mut Request| -> IronResult<Response> {
                        postlog(req, &auditor)
                    }, "log");
                    router.get("/api/v1/log", move |req: &mut Request| -> IronResult<Response> {
                        getlog(req, &auditor)
                    }, "log");
                    Iron::new(router)
                        .http(format!("localhost:{}", server_options.port))
                        .unwrap();
                }
                _ => {
                    panic!("inpossible code");
                }
            }
        }
        Command::Debugger(_) => {

            let conn = connect_to_db(&auditor);

            for row in &conn.query("SELECT insertion_time, name, dict from monitoring.event", &[]).unwrap() {
                let event = Event{
                    insertion_time: row.get(0),
                    d: EventIngest {
                        name: row.get(1),
                        dict: row.get(2)
                    }
                };
                println!("event: {:?}", event);
            }
        }
        Command::CliClient(_) => {
            auditor.tell(&audit::Concern::Crisis(audit::Event::new("error", "true")));
        }
    }





    /*

Connection pooler

maek this workz
let manager = PostgresConnectionManager::new(
        "host=localhost user=postgres password=aaargh port=5432 database=postgres".parse().unwrap(),
        NoTls,
    );
    let pool = r2d2_postgres::Pool::new(manager).unwrap();
     */

}
