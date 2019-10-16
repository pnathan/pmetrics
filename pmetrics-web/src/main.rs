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

// TODO: Api should have an accept: application/json request type, along with appropriate error codes.
// Said error codes will be:
//
// 400 YOU SEND ME A BAD THING, CANT UNDERSTAND IT. MAYBE BETTER JSON NEXT TIME?
// 422 GOOD REQUEST SYNTAX, BUT DIDN'T MAKE SENSE. WATFACE
// 500 BARF
// 502 COULDN'T TALK TO DATABASE.

// Technicall 201 should be the typical response for a POST here.

// In semantic, the GETs for the API are essentially tailing the stream.

// TODO: add middleware specifying the specifics of the ip connection


// TODO: Write a Search api.

/*
fn generic_post<'a, T>(req: & mut Request,
                       auditor: &audit::Audit,
                       f: dyn Fn(postgres::Connection, T)->Result<u64, postgres::Error>) -> IronResult<Response>
where T: serde::Deserialize<'a> {

    let conn = connect_to_db(&auditor);
    let mut buffer = String::new();
    req.body.read_to_string(&mut buffer);
    let v: Result<T, serde_json::Error> = serde_json::from_str(&buffer);
    match v {
        Ok(deserialized) => {
            match f(conn, deserialized) {
                Ok(_) => {
                    Ok(Response::with((status::Ok, "ok")))
                }
                Err(_) => {
                    auditor.debug(audit::event("error", "true"));
                    Ok(Response::with((status::BadGateway, "server error")))
                }
            }
        }

        Err(_) => {
            auditor.debug(audit::event("error", "true"));
            Ok(Response::with((status::BadRequest, "bad parse and cast")))
        }
    }
}

fn bigpost(req: &mut Request, auditor: &audit::Audit) -> IronResult<Response> {
    fn logf(conn: postgres::Connection, l: LogIngest) -> Result<u64, postgres::Error> {
        conn.execute("INSERT INTO monitoring.log (logtext) VALUES ($1)",
                     &[&l.log])

    };
    generic_post(req, auditor, logf)
}
*/

fn postlog(req: &mut Request, auditor: &audit::Audit) -> IronResult<Response> {

    let conn = connect_to_db(&auditor);
    let mut buffer = String::new();
    req.body.read_to_string(&mut buffer);
    let v: Result<LogIngest, serde_json::Error> = serde_json::from_str(&buffer);
    match v {
        Ok(deserialized) => {
            match conn.execute("INSERT INTO monitoring.log (logtext) VALUES ($1)",
                               &[&deserialized.log]) {
                Ok(_) => {
                    Ok(Response::with((status::Ok, "ok")))
                }
                Err(_) => {
                    auditor.debug(audit::event("error", "true"));
                    Ok(Response::with((status::BadGateway, "server error")))
                }
            }
        }

        Err(_) => {
            auditor.debug(audit::event("error", "true"));
            Ok(Response::with((status::BadRequest, "bad parse and cast")))
        }
    }
}

fn getlog(req: &mut Request, auditor: &audit::Audit) -> IronResult<Response> {
    let conn = connect_to_db(&auditor);
    let mut vec: Vec<Log> = Vec::new();
    match conn.query("SELECT insertion_time, logtext from monitoring.log order by insertion_time desc limit 100", &[]) {
        Ok(rows) => {
            for row in &rows {
                vec.push(Log {
                    insertion_time: row.get(0),
                    d: LogIngest {
                        log: row.get(1),
                    }
                });
            }

            // This should not be a problematic unwrap... right...?
            let result = serde_json::to_string(&vec).unwrap();
            Ok(Response::with((status::Ok, result)))
        }
        Err(e) => {
            Ok(Response::with((status::InternalServerError, "server error, can't get data")))
        }
    }
}


fn postmeasure(req: &mut Request, auditor: &audit::Audit) -> IronResult<Response> {
    let conn = connect_to_db(&auditor);
    auditor.debug(audit::event("started", "http connection"));
    let mut buffer = String::new();
    req.body.read_to_string(&mut buffer);
    let deserialized: MeasureIngest = serde_json::from_str(&buffer).unwrap();
    conn.execute("INSERT INTO monitoring.measure (name, measurement, dict) VALUES ($1, $2, $3)",
                 &[&deserialized.name, &deserialized.measurement, &deserialized.dict]).unwrap();
    auditor.debug(audit::event("ending", "http connection"));
    Ok(Response::with((status::Ok, "ok")))
}

fn getmeasure(req: &mut Request, auditor: &audit::Audit) -> IronResult<Response> {
    let conn = connect_to_db(&auditor);
    auditor.debug(audit::event("started", "http connection"));

    let mut vec: Vec<Measure> = Vec::new();

    for row in &conn.query("SELECT insertion_time, name, measurement, dict from monitoring.measure order by insertion_time desc limit 100", &[]).unwrap() {
        vec.push(Measure {
            insertion_time: row.get(0),
            d: MeasureIngest {
                name: row.get(1),
                measurement: row.get(2),
                dict: row.get(3)
            }
        });
    }

    let result = serde_json::to_string(&vec).unwrap();
    auditor.debug(audit::event("ending", "http connection"));
    Ok(Response::with((status::Ok, result)))
}

fn postevent(req: &mut Request, auditor: &audit::Audit) -> IronResult<Response> {
    let conn = connect_to_db(&auditor);
    let mut buffer = String::new();
    req.body.read_to_string(&mut buffer);
    let deserialized: EventIngest = serde_json::from_str(&buffer).unwrap();
    conn.execute("INSERT INTO monitoring.event (name, dict) VALUES ($1, $2)", &[&deserialized.name, &deserialized.dict]).unwrap();
    auditor.debug(audit::Event::new("event", "inserted"));
    Ok(Response::with((status::Ok, "ok")))
}

fn getevent(req: &mut Request, auditor: &audit::Audit) -> IronResult<Response> {
    let conn = connect_to_db(&auditor);
    auditor.debug(audit::event("started", "http connection"));

    let mut vec: Vec<Event> = Vec::new();

    for row in &conn.query("SELECT insertion_time, name, dict from monitoring.event order by insertion_time desc limit 100", &[]).unwrap() {
        vec.push(Event {
            insertion_time: row.get(0),
            d: EventIngest {
                name: row.get(1),
                dict: row.get(2)
            }
        });
    }

    let result = serde_json::to_string(&vec).unwrap();
    auditor.debug(audit::event("ending", "http connection"));
    Ok(Response::with((status::Ok, result)))
}

fn handler(req: &mut Request) -> IronResult<Response> {

    //let ref query = req.extensions.get::<Router>().unwrap().find("query").unwrap_or("/");
    Ok(Response::with((status::Ok, "welcome to pmetrics")))
}

fn launch_server(cl: &audit::ConcernLevel, server_options: &ServerOptions) {
    let auditor  = audit::Audit::new(*cl);

    let mut router = Router::new();
    router.get("/", handler, "index");

    router.post("/api/v1/event", move |req: &mut Request| -> IronResult<Response> {
        postevent(req, &auditor)
    }, "event");
    router.get("/api/v1/event", move |req: &mut Request| -> IronResult<Response> {
        getevent(req, &auditor)
    }, "event");


    router.post("/api/v1/log", move |req: &mut Request| -> IronResult<Response> {
        postlog(req, &auditor)
    }, "log");
    router.get("/api/v1/log", move |req: &mut Request| -> IronResult<Response> {
        getlog(req, &auditor)
    }, "log");

    router.post("/api/v1/measure", move |req: &mut Request| -> IronResult<Response> {
        postmeasure(req, &auditor)
    }, "measure");
    router.get("/api/v1/measure", move |req: &mut Request| -> IronResult<Response> {
        getmeasure(req, &auditor)
    }, "measure");

    Iron::new(router)
        .http(format!("localhost:{}", server_options.port))
        .unwrap();
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


fn connect_to_db(auditor: &audit::Audit) -> postgres::Connection {
    let cs = PostgresClientArgs::from_env().connection_string();
    println!("CS: {}", &cs);
    auditor.tell(&audit::Concern::Info(audit::Event::new("starting", "pg conn")));
    let conn = Connection::connect(cs, TlsMode::None).unwrap();
    auditor.tell(&audit::Concern::Info(audit::Event::new("started", "pg conn")));

    conn
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
                "json" => {
                    Command::Server(go,
                                    ServerOptions { port: 1337 },
                                    ServerType::Http)
                }
                _ => {
                    panic!("I only speak `json` server, not {}", servertype);
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
    let cmd = cliparser();

    let cl = match cmd {
        Command::Server(go, _, _) => go.verbosity,
        Command::Debugger(go) => go.verbosity,
        Command::CliClient(go) => go.verbosity
    };

    match cmd {
        Command::Server(_, server_options, servertype) => {
            match servertype {
                ServerType::Http => launch_server(&cl, &server_options),
                    _ =>  panic!("inpossible code")
            }
        }

        Command::Debugger(_) => {

            let auditor  = audit::Audit::new(cl);
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
            // we should take STDIN or PIPE events
            println!("CLI NOT IMPLEMENTED");

        }
    }
}
