/**
pmetrics entry point
 **/
#[macro_use]
extern crate nickel;

use clap::{Parser, Subcommand};
use log::LevelFilter;
use std::fs::File;
use std::io;
use std::io::Read;
use std::{thread, time};

use chrono::prelude::{DateTime, Utc};

use nickel::status::StatusCode;
use nickel::{/* QueryString, */ HttpRouter, MiddlewareResult, Nickel, Request, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
// this is /not obvious/.
use pmetrics::db;
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MeasureIngest {
    name: String,
    measurement: f64,
    dict: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct IntrusiveMeasure {
    insertion_time: DateTime<Utc>,
    name: String,
    measurement: f64,
    dict: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct Measure {
    d: MeasureIngest,
    insertion_time: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct EventIngest {
    name: String,
    dict: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct IntrusiveEvent {
    insertion_time: DateTime<Utc>,
    name: String,
    dict: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct Event {
    d: EventIngest,
    insertion_time: DateTime<Utc>,
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

fn generic_post<T, F>(req: &mut Request, insert_function: F) -> (nickel::status::StatusCode, String)
where
    T: DeserializeOwned,
    F: Fn(&mut postgres::Client, &T) -> Result<u64, postgres::Error>,
{
    let mut buffer = String::new();

    match req.origin.read_to_string(&mut buffer) {
        Ok(_) => {} // no-op
        Err(_) => {
            log::info!("error=true module=web class=string_read");
            return (StatusCode::BadRequest, "unable to read string".to_string());
        }
    }

    let v: Result<T, serde_json::Error> = serde_json::from_str(&buffer);
    match v {
        Ok(deserialized) => {
            let mut conn = db::connect_to_db();
            match insert_function(&mut conn, &deserialized) {
                Ok(_) => (StatusCode::Ok, "ok".to_string()),
                Err(err) => {
                    log::debug!("error=true module=web class=db_insert details={}", err);
                    log::info!("error=true module=web class=db_insert");
                    (StatusCode::BadGateway, "server error".to_string())
                }
            }
        }

        Err(_) => {
            log::info!("error=true module=web class=deserialize/parse");
            (StatusCode::BadRequest, "bad parse and cast".to_string())
        }
    }
}

fn writemeasure(
    conn: &mut postgres::Client,
    tid: i32,
    l: &MeasureIngest,
) -> Result<u64, postgres::Error> {
    conn.execute("INSERT INTO monitoring.measure (name, tenant_id, measurement, dict) VALUES ($1, $2, $3, $4)",
                 &[&l.name, &tid, &l.measurement, &l.dict])
}

fn postmeasure(req: &mut Request) -> (nickel::status::StatusCode, String) {
    match get_tid(req) {
        Some(tid) => {
            let f = |conn: &mut postgres::Client,
                     l: &MeasureIngest|
             -> Result<u64, postgres::Error> { writemeasure(conn, tid, l) };

            generic_post(req, f)
        }
        None => {
            log::info!(
                "error=true module=web what='failed to get the x tenant id from the middleware'"
            );
            (StatusCode::BadRequest, "\"key failure\"".to_string())
        }
    }
}

// TODO: dry up.
fn getmeasure(req: &mut Request) -> (nickel::status::StatusCode, String) {
    let mut conn = db::connect_to_db();
    let tid = get_tid(req).unwrap();

    let query = "SELECT insertion_time, name, measurement, dict from monitoring.measure where tenant_id = $1 order by insertion_time desc limit 100";
    match conn.query(query, &[&tid]) {
        Ok(rows) => {
            let mut vec: Vec<Measure> = Vec::new();
            for row in &rows {
                vec.push(Measure {
                    insertion_time: row.get(0),
                    d: MeasureIngest {
                        name: row.get(1),
                        measurement: row.get(2),
                        dict: row.get(3),
                    },
                });
            }
            let result = serde_json::to_string(&vec).unwrap();
            (StatusCode::Ok, result)
        }
        Err(e) => {
            log::error!("error=true module=db error={} query={}", e, &query);
            (
                StatusCode::InternalServerError,
                "server error, can't get data".to_string(),
            )
        }
    }
}

fn writeevent(
    conn: &mut postgres::Client,
    tid: i32,
    l: &EventIngest,
) -> Result<u64, postgres::Error> {
    conn.execute(
        "INSERT INTO monitoring.event (name, tenant_id, dict) VALUES ($1, $2, $3)",
        &[&l.name, &tid, &l.dict],
    )
}

fn postevent(req: &mut Request) -> (nickel::status::StatusCode, String) {
    match get_tid(req) {
        Some(tid) => {
            let f = |conn: &mut postgres::Client,
                     l: &EventIngest|
             -> Result<u64, postgres::Error> { writeevent(conn, tid, l) };

            generic_post(req, f)
        }
        None => {
            log::info!(
                "error=true module=web what='failed to get the x tenant id from the middleware'"
            );
            (StatusCode::BadRequest, "\"key failure\"".to_string())
        }
    }
}

fn getevent(req: &mut Request) -> (nickel::status::StatusCode, String) {
    let mut conn = db::connect_to_db();
    let mut vec: Vec<Event> = Vec::new();
    let tid = get_tid(req).unwrap();

    for row in &conn.query("SELECT insertion_time, name, dict from monitoring.event where tenant_id = $1 order by insertion_time desc limit 100",
                           &[&tid]).unwrap() {
        vec.push(Event {
            insertion_time: row.get(0),
            d: EventIngest {
                name: row.get(1),
                dict: row.get(2),
            },
        });
    }

    let result = serde_json::to_string(&vec).unwrap();
    (StatusCode::Ok, result)
}

// index /
fn handler(_req: &mut Request) -> (nickel::status::StatusCode, String) {
    (StatusCode::Ok, "welcome to nickel'd pmetrics".to_string())
}

// healthz - am I alive?
// does not check database liveness though.
fn healthz(_req: &mut Request) -> (nickel::status::StatusCode, String) {
    log::info!("healthz");
    (StatusCode::Ok, "ok".to_string())
}

//////////////////////////////
// API KEY / tenant middleware.

fn get_tid(req: &Request) -> Option<i32> {
    match req.origin.headers.get_raw("X-TENANT-ID") {
        Some(s) => {
            let thread_string = s[0].to_vec();
            let tid: i32 = String::from_utf8(thread_string).unwrap().parse().unwrap();
            Some(tid)
        }
        None => {
            log::info!(
                "error=true module=web what='failed to get the x tenant id from the middleware'"
            );
            None
        }
    }
}

struct ApiKeys;

impl ApiKeys {
    fn check_keys(&self, k: &str) -> Option<i32> {
        // reads monitoring.apikeys
        let mut conn = db::connect_to_db();
        let mut vec: Vec<i32> = Vec::new();

        for row in &conn
            .query(
                "SELECT uid from monitoring.tenant where apikey = $1",
                &[&k.to_string()],
            )
            .unwrap()
        {
            vec.push(row.get(0));
        }

        if !vec.is_empty() {
            Some(vec[0])
        } else {
            None
        }
    }
}

fn check_api_keys<'mw>(_req: &mut Request, mut res: Response<'mw>) -> MiddlewareResult<'mw> {
    let path = _req.path_without_query().unwrap();
    // Cutout for non-api routes.
    if !path.contains("api") {
        return res.next_middleware();
    }

    match _req.origin.headers.get_raw("X-PMETRICS-API-KEY") {
        Some(s) => {
            let gatekeeper = ApiKeys {};
            let header = &s[0];
            let key: String = String::from_utf8(header.to_vec()).unwrap();
            let apikeys = gatekeeper.check_keys(&key);
            match apikeys {
                Some(v) => {
                    // Set it for other users lower down in the middleware stack.
                    _req.origin
                        .headers
                        .set_raw("X-TENANT-ID", vec![v.to_string().into_bytes()]);
                }
                None => {
                    res.set(StatusCode::Forbidden);

                    return res.send("\"api key failure\"");
                }
            }
        }
        None => {
            res.set(StatusCode::Forbidden);

            return res.send("\"api key failure\"");
        }
    }

    // Pass control to the next middleware
    res.next_middleware()
}

fn log_request<'mw>(_req: &mut Request, res: Response<'mw>) -> MiddlewareResult<'mw> {
    match _req.origin.headers.get_raw("X-PMETRICS-API-KEY") {
        Some(key) => {
            let header = &key[0];
            let key: String = String::from_utf8(header.to_vec()).unwrap();
            log::info!(
                "module=web method={} url={} apikey={}",
                &_req.origin.method.to_string(),
                _req.path_without_query().unwrap(),
                &key,
            );
        }
        None => {
            log::info!(
                "module=web method={} url={}",
                &_req.origin.method.to_string(),
                _req.path_without_query().unwrap(),
            );
        }
    }
    res.next_middleware()
}

fn launch_server(server_options: &ServerOptions) {
    log::info!("message='server initializing'");
    let mut server = Nickel::new();

    server.get(
        "/healthz",
        middleware! { |req|
                                              healthz(req)
        },
    );

    server.utilize(check_api_keys);

    server.get(
        "/",
        middleware! { |req|
                                      handler(req)
        },
    );

    server.utilize(log_request);

    server.post(
        "/api/v1/event",
        middleware! { |req|
            postevent(req)
        },
    );
    server.get(
        "/api/v1/event",
        middleware! { |req|
            getevent(req)
        },
    );

    server.post(
        "/api/v1/measure",
        middleware! { |req|
            postmeasure(req)
        },
    );

    server.get(
        "/api/v1/measure",
        middleware! { |req|
            getmeasure(req)
        },
    );

    log::info!("server starting on port {}", server_options.port);

    server
        .listen(format!("0.0.0.0:{}", server_options.port))
        .unwrap();
}

fn launch_query(qo: &QueryOptions) {
    let mut conn = db::connect_to_db();
    println!("{qo:?}");
    // pg / rust-postgres demand i64 as the type to be passed in.
    let last: i64 = qo.last.into();
    let printable = match qo.metric_type {
        MetricTypeOption::E => {
            let mut vec: Vec<IntrusiveEvent> = Vec::new();
            let query = "SELECT insertion_time, name, dict from monitoring.event
order by insertion_time desc
limit $1";
            for row in &conn.query(query, &[&last]).unwrap() {
                vec.push(IntrusiveEvent {
                    insertion_time: row.get(0),
                    name: row.get(1),
                    dict: row.get(2),
                });
            }
            serde_json::to_string_pretty(&vec).unwrap()
        }
        MetricTypeOption::M => {
            let mut vec: Vec<IntrusiveMeasure> = Vec::new();
            let query = "SELECT insertion_time, name, measurement, dict
from monitoring.measure
order by insertion_time desc
limit $1";
            for row in &conn.query(query, &[&last]).unwrap() {
                vec.push(IntrusiveMeasure {
                    insertion_time: row.get(0),
                    name: row.get(1),
                    measurement: row.get(2),
                    dict: row.get(3),
                });
            }

            serde_json::to_string_pretty(&vec).unwrap()
        }
    };

    println!("{printable}");
}

#[derive(Debug, Serialize, Deserialize)]
enum PipeReader {
    M(MeasureIngest),
    E(EventIngest),
}

fn launch_writer(filename: String, apikey: String) {
    let mut conn = db::connect_to_db();

    let mut file: Box<dyn Read> = match filename.as_str() {
        "-" => Box::new(io::stdin()),
        // crashing here is ok if we can't open it.
        _ => {
            log::info!("opening {}", &filename);
            Box::new(File::open(filename).unwrap())
        }
    };

    let gatekeeper = ApiKeys {};
    let tid: i32 = match gatekeeper.check_keys(&apikey) {
        Some(i) => i,
        None => {
            log::info!("api key failure {}", &apikey);
            panic!("api key didn't work");
        }
    };

    loop {
        let mut buffer = String::new();

        // this frankly should be epoll based for a named pipe, but
        // let's let it live for now. If this is a _useful_ system, we
        // can do more with it.
        let result = file.read_to_string(&mut buffer);
        match result {
            Ok(bytecount) => {
                if bytecount > 0 {
                    log::info!("status=rx");
                    let v: Result<Vec<PipeReader>, serde_json::Error> =
                        serde_json::from_str(&buffer);

                    match v {
                        Ok(dataz) => {
                            for row in &dataz {
                                match row {
                                    PipeReader::M(measure) => {
                                        writemeasure(&mut conn, tid.clone(), measure).unwrap();
                                    }
                                    PipeReader::E(event) => {
                                        writeevent(&mut conn, tid.clone(), event).unwrap();
                                    }
                                }
                                log::info!("status=written");
                            }
                        }
                        Err(e) => {
                            log::error!("err={:?}", e);
                        }
                    }
                }
            }
            Err(e) => {
                log::info!("err={:?}", e);
            }
        }

        // One second pulled out of thin air.
        let one = time::Duration::from_secs(1);
        thread::sleep(one);
    }
}

#[derive(Debug)]
enum ServerType {
    Http,
}

#[derive(Debug)]
struct CliOptions {
    filename: String,
    apikey: String,
}

#[derive(Debug)]
struct ServerOptions {
    port: u16,
}

#[derive(Debug)]
enum MetricTypeOption {
    M,
    E,
}

#[derive(Debug)]
struct QueryOptions {
    metric_type: MetricTypeOption,
    last: u16,
}

#[derive(Debug)]
enum Command {
    PipeReader(CliOptions),
    Server(ServerOptions, ServerType),
    Querier(QueryOptions),
}

#[derive(Debug, Subcommand)]
enum PmetricsMode {
    Pipe {
        #[arg(short, long)]
        file: String,
        #[arg(short, long)]
        api_key: String,
    },
    Server {
        #[arg(short, long)]
        server_type: String,
        #[arg(short, long)]
        port: u16,
    },
    Querier {
        metric_type: String,
        last: u16,
    },
}

#[derive(Debug, Parser)]
#[command(name = "pmetrics")]
#[command(version = "1.0.1")]
#[command(about = "an observability system", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(short, long, default_value = "1", help = "verbosity level")]
    v: u8,
    #[command(subcommand)]
    cmd: PmetricsMode,
}

fn clapparser() -> (Command, u8) {
    let cli = Cli::parse();

    let cmd = match cli.cmd {
        PmetricsMode::Server { server_type, port } => {
            let st = match server_type.as_str() {
                "http" => ServerType::Http,
                _ => panic!("Unable to start server, crashing. specify type http"),
            };

            let so = ServerOptions { port };

            Command::Server(so, st)
        }
        PmetricsMode::Pipe { file, api_key } => Command::PipeReader(CliOptions {
            filename: file,
            apikey: api_key,
        }),
        PmetricsMode::Querier {
            metric_type, last, ..
        } => {
            let mt = match metric_type.as_str() {
                "m" => MetricTypeOption::M,
                "e" => MetricTypeOption::E,
                _ => panic!("not a valid metric type - try m or e"),
            };

            let qo = QueryOptions {
                metric_type: mt,
                last,
            };
            Command::Querier(qo)
        }
    };
    (cmd, cli.v)
}

fn main() {
    let (cmd, verbosity) = clapparser();

    let log_level = match verbosity {
        0 => LevelFilter::Error,
        1 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };

    env_logger::Builder::new().filter_level(log_level).init();

    match cmd {
        Command::Server(server_options, servertype) => match servertype {
            ServerType::Http => launch_server(&server_options),
        },
        Command::PipeReader(clioptions) => launch_writer(clioptions.filename, clioptions.apikey),
        Command::Querier(qo) => launch_query(&qo),
    }
}
