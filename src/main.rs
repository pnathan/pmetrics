extern crate chrono;
extern crate iron;
extern crate router;
extern crate serde;
extern crate serde_json;
extern crate postgres;
extern crate clap;


use std::io;
use std::io::{Read};
use std::fs::File;
use chrono::prelude::{DateTime, Utc};
use clap::{Arg, App, SubCommand};
use iron::prelude::{IronResult, Request, Response, Iron};
use iron::status;
use postgres::Connection;
use router::Router;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned; // this is /not obvious/.
use serde_json::{Value};

mod db;
mod audit;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MeasureIngest {
    name: String,
    measurement: f64,
    dict: Value
}

#[derive(Debug, Serialize, Deserialize)]
struct Measure {
    d: MeasureIngest,
    insertion_time: DateTime<Utc>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct EventIngest {
    name: String,
    dict: Value
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

fn generic_post<'a, T, F>(req: &'a mut Request,
                          auditor: &audit::Audit,
                          insert_function: F) -> IronResult<Response>
where T: DeserializeOwned,
      F: Fn(&postgres::Connection, &T)->Result<u64, postgres::Error>,
{
    let mut buffer = String::new();

    match req.body.read_to_string(&mut buffer) {
        Ok(_) => {} // no-op
        Err(_)=> {
            auditor.info(audit::eventw(&["error", "true", "module", "web", "class", "string read"]));
            return Ok(Response::with((status::BadRequest, "unable to read string")))
        }
    }
    let v: Result<T, serde_json::Error> = serde_json::from_str(&buffer);
    match v {
        Ok(deserialized) => {
            let conn = db::connect_to_db(&auditor);
            match insert_function(&conn, &deserialized) {
                Ok(_) => {
                    Ok(Response::with((status::Ok, "ok")))
                }
                Err(_) => {
                    auditor.info(audit::eventw(&["error", "true","module", "web",  "class", "db insert"]));
                    Ok(Response::with((status::BadGateway, "server error")))
                }
            }
        }

        Err(_) => {
            auditor.info(audit::eventw(&["error", "true", "module", "web",  "class", "deserialize/parse"]));
            Ok(Response::with((status::BadRequest, "bad parse and cast")))
        }
    }
}

fn writemeasure(conn: &postgres::Connection, l: &MeasureIngest) -> Result<u64, postgres::Error> {
    conn.execute("INSERT INTO monitoring.measure (name, measurement, dict) VALUES ($1, $2, $3)",
                 &[&l.name, &l.measurement, &l.dict])

}
fn postmeasure(req: &mut Request, auditor: &audit::Audit) -> IronResult<Response> {

    let f = |conn: &postgres::Connection, l: &MeasureIngest| -> Result<u64, postgres::Error> {
        writemeasure(conn, l)
    };

    generic_post(req, auditor, f)
}

// TODO: dry up.
fn getmeasure(_req: &mut Request, auditor: &audit::Audit) -> IronResult<Response> {
    let conn = db::connect_to_db(&auditor);
    let query = "SELECT insertion_time, name, measurement, dict from monitoring.measure order by insertion_time desc limit 100";
    match conn.query(query, &[]) {
        Ok(rows) => {
            let mut vec: Vec<Measure> = Vec::new();
            for row in &rows {
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
            Ok(Response::with((status::Ok, result)))
        }
        Err(e) => {
            auditor.crisis(audit::eventw(&["error", "true", "module", "db", "error", format!("{:?}", e).as_str(), "query", &query]));
            Ok(Response::with((status::InternalServerError, "server error, can't get data")))
        }
    }
}

fn writeevent(conn: &postgres::Connection, l: &EventIngest) -> Result<u64, postgres::Error> {
    conn.execute("INSERT INTO monitoring.event (name, dict) VALUES ($1, $2)",
                 &[&l.name, &l.dict])
}

fn postevent(req: &mut Request, auditor: &audit::Audit) -> IronResult<Response> {

    let f = |conn: &postgres::Connection, l: &EventIngest| -> Result<u64, postgres::Error> {
        writeevent(conn, l)

    };

    generic_post(req, auditor, f)
}

fn getevent(_req: &mut Request, auditor: &audit::Audit) -> IronResult<Response> {
    let conn = db::connect_to_db(&auditor);
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

fn handler(_req: &mut Request) -> IronResult<Response> {
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

    router.post("/api/v1/measure", move |req: &mut Request| -> IronResult<Response> {
        postmeasure(req, &auditor)
    }, "measure");
    router.get("/api/v1/measure", move |req: &mut Request| -> IronResult<Response> {
        getmeasure(req, &auditor)
    }, "measure");


    auditor.info(audit::event("server starting", &format!("{}", server_options.port)));

    Iron::new(router)
        .http(format!("localhost:{}", server_options.port))
        .unwrap();
}


#[derive(Debug, Serialize, Deserialize)]
enum PipeReader {
    M(MeasureIngest),
    E(EventIngest)
}


fn launch_writer(cl: &audit::ConcernLevel, filename: String) {
    let auditor  = audit::Audit::new(*cl);
    let conn = db::connect_to_db(&auditor);

    let mut file = match filename.as_str() {
        "-" => io::stdin(),
        // crashing here is ok if we can't open it.
        _ => File::open(filename).unwrap()
    };


    loop {
        let mut buffer = String::new();
        let result = file.read_to_string(&mut buffer);

        match result {
          Ok(bytecount) =>
                if bytecount > 0 {
                    let v: Result<Vec<PipeReader>, serde_json::Error> = serde_json::from_str(&buffer);

                    match v {
                        Ok(dataz) => {
                            for row in &dataz {
                                match row {
                                    PipeReader::M(measure) => {
                                        writemeasure(&conn,measure);
                                    }
                                    PipeReader::E(event) => {
                                        writeevent(&conn, event);
                                    }
                                }
                                auditor.info(audit::event("writes", "I did it!!"));
                            }
                        }
                        Err(e) => {
                            println!("{:?}", e);
                        }
                    }
                }
            Err(e) => {
                auditor.info(audit::event("err", &format!("{:?}", e)));
            }
        }

    }
}

#[derive(Copy, Clone)]
struct GlobalOptions {
    verbosity: audit::ConcernLevel
}

enum ServerType {
    Http
}
struct CliOptions {
    filename: String
}
struct ServerOptions {
    port: u16
}

enum Command {
    CliClient(GlobalOptions, CliOptions),
    Server(GlobalOptions, ServerOptions, ServerType),
    Debugger(GlobalOptions)
}

fn clapparser() -> Command {
    let matches = App::new("pmetrics")
        .version("0.0.1")
        .about("an o11y system")
        .arg(Arg::with_name("v")
             .short("v")
             .multiple(true)
             .help("verbosity - 0,1,2 being valid values "))
        .subcommand(SubCommand::with_name("server")
                    .about("server")
                    .arg(Arg::with_name("type")
                         .required(true)
                         .help("type of server"))
                    .arg(Arg::with_name("port")
                         .short("p")
                         .takes_value(true)
                         .help("server port")))
        .subcommand(SubCommand::with_name("cliput")
                    .about("cli putter. waits on STDIN for json blocks to be sent")
                    .arg(Arg::with_name("file")
                         .short("f")
                         .takes_value(true)
                         .help("file: can be ordinary or a named pipe. STDIN is used when not specified")))
        .get_matches();

    let cl = match matches.occurrences_of("v") {
        0 => audit::ConcernLevel::Crisis,
        1 => audit::ConcernLevel::Info,
        2 | _ => audit::ConcernLevel::Debug
    };

    let go = GlobalOptions { verbosity: cl };

    if let Some(matches) = matches.subcommand_matches("server") {

        let servertype = matches.value_of("type").unwrap();
        let st = match servertype  {
            "http" => ServerType::Http,
            _ => panic!("Unable to start server, crashing. specify type http")
        };

        let port = matches.value_of("port").unwrap_or("1337").parse::<u16>().unwrap();
        let so = ServerOptions { port: port };

        return Command::Server(go, so, st)
    }

    if let Some(_matches) = matches.subcommand_matches("cliput") {
        let filename = matches.value_of("file").unwrap_or("-");
        return Command::CliClient(go, CliOptions { filename: filename.to_string() })
    }

    panic!("what! run --help please!")
}

fn main() {
    let cmd = clapparser();

    let cl = match cmd {
        Command::Server(go, _, _) => go.verbosity,
        Command::Debugger(go) => go.verbosity,
        Command::CliClient(go, _) => go.verbosity
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
            let conn = db::connect_to_db(&auditor);

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
        Command::CliClient(_, clioptions) => {
            launch_writer(&cl, clioptions.filename)
        }
    }
}
